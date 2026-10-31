use super::super::SYSTEM_PALETTE;
use super::*;

fn fixture() -> Ppu {
    let mut p = Ppu::new();
    p.spr_ram = [0xff; 256];
    p.mask = 0x1e;
    for &(addr, color) in &[
        (0x3f00, 0x0f),
        (0x3f01, 1),
        (0x3f11, 0x16),
        (0x3f12, 0x1a),
        (0x3f13, 0x30),
    ] {
        p.vram.write(addr, color);
    }
    p
}

fn put_sprite(p: &mut Ppu, index: u8, y: u8, tile: u8, attributes: u8, x: u8) {
    p.write_register(3, index * 4);
    for value in &[y, tile, attributes, x] {
        p.write_register(4, *value);
    }
}

fn prepare_row(p: &mut Ppu, line: i16) {
    p.scanline = line - 1;
    p.cycle = 1;
    for _ in 0..320 {
        p.step();
    }
    p.scanline = line;
    p.cycle = 1;
}

fn pattern(tile: u8, bank: u8, row: u8, column: u8) -> u8 {
    (tile
        .wrapping_add(bank * 3)
        .wrapping_add(row * 2)
        .wrapping_add(column))
        & 3
}

fn load_patterns(p: &mut Ppu) {
    for bank in 0..2u8 {
        for tile in 0..256u16 {
            for row in 0..8u8 {
                let (mut low, mut high) = (0, 0);
                for column in 0..8u8 {
                    let color = pattern(tile as u8, bank, row, column);
                    low = (low << 1) | (color & 1);
                    high = (high << 1) | (color >> 1);
                }
                let addr = bank as u16 * 0x1000 + tile * 16 + row as u16;
                p.vram.write(addr, low);
                p.vram.write(addr + 8, high);
            }
        }
    }
}

#[test]
fn sprite_rows_cover_sizes_banks_flips_and_all_palettes() {
    let mut p = fixture();
    load_patterns(&mut p);
    for tall in &[false, true] {
        for bank in &[0, 8] {
            for tile in &[2, 3, 0xfe, 0xff] {
                for flip in &[0, 0x40, 0x80, 0xc0] {
                    for palette in 0..4u8 {
                        p.ctrl = if *tall { 0x20 | bank } else { *bank };
                        put_sprite(&mut p, 0, 10, *tile, flip | palette, 16);
                        let height = if *tall { 16 } else { 8 };
                        for row in 0..height {
                            prepare_row(&mut p, 11 + row as i16);
                            assert_eq!(p.sprites.active_count, 1);
                            for column in 0..8 {
                                p.cycle = 17 + column as u16;
                                let source_row = if flip & 0x80 != 0 {
                                    height - 1 - row
                                } else {
                                    row
                                };
                                let source_column =
                                    if flip & 0x40 != 0 { 7 - column } else { column };
                                let source_bank = if *tall { tile & 1 } else { bank >> 3 };
                                let source_tile = if *tall {
                                    (tile & 0xfe) + source_row / 8
                                } else {
                                    *tile
                                };
                                let expected = pattern(
                                    source_tile,
                                    source_bank,
                                    source_row & 7,
                                    source_column,
                                );
                                let actual = p.sprite_pixel();
                                if expected == 0 {
                                    assert!(actual.is_none());
                                } else {
                                    let actual = actual.unwrap();
                                    assert_eq!(actual.palette, 0x10 | palette * 4 | expected);
                                    assert!(actual.zero);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn sprite_pixel_bitplanes_are_exhaustive_in_both_horizontal_directions() {
    let mut p = fixture();
    p.sprites.active_count = 1;
    for low in 0..256u16 {
        for high in 0..256u16 {
            for flip in &[0, 0x40] {
                p.sprites.active[0] = Sprite {
                    x: 8,
                    attributes: *flip,
                    low: low as u8,
                    high: high as u8,
                    zero: true,
                };
                for column in 0..8u16 {
                    p.cycle = 9 + column;
                    let bit = if *flip == 0 { 7 - column } else { column };
                    let color = ((low >> bit) & 1) | (((high >> bit) & 1) << 1);
                    match p.sprite_pixel() {
                        None => assert_eq!(color, 0),
                        Some(pixel) => {
                            assert_ne!(color, 0);
                            assert_eq!(pixel.palette, 0x10 | color as u8);
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn evaluation_selects_the_first_eight_in_oam_order_including_late_entries() {
    let mut p = fixture();
    for &index in &[2, 7, 12, 18, 27, 36, 50, 61, 63] {
        put_sprite(&mut p, index, 20, 1, 0, index);
    }
    prepare_row(&mut p, 21);
    assert_eq!(p.sprites.active_count, 8);
    assert_eq!(p.sprites.indices, [2, 7, 12, 18, 27, 36, 50, 61]);
    assert!(p.sprites.active.iter().all(|s| !s.zero));
}

#[test]
fn transparent_and_clipped_sprites_still_use_evaluation_slots() {
    let mut p = fixture();
    for index in 0..8 {
        put_sprite(&mut p, index, 20, 0, 0, 255);
    }
    put_sprite(&mut p, 8, 20, 1, 0, 16);
    for row in 0..8 {
        p.vram.write(16 + row, 0xff);
    }
    prepare_row(&mut p, 21);
    assert_eq!(p.sprites.active_count, 8);
    p.cycle = 17;
    assert!(p.sprite_pixel().is_none());
    assert_ne!(p.status & STATUS_SPRITE_OVERFLOW, 0);
}

#[test]
fn sprites_use_y_plus_one_without_top_or_bottom_wrap() {
    let mut p = fixture();
    p.vram.write(16, 0xff);
    put_sprite(&mut p, 0, 0, 1, 0, 16);
    prepare_row(&mut p, 0);
    assert_eq!(p.sprites.active_count, 0);
    prepare_row(&mut p, 1);
    p.cycle = 17;
    assert_eq!(p.sprite_pixel().unwrap().palette, 0x11);
    for y in 239..256u16 {
        put_sprite(&mut p, 0, y as u8, 1, 0, 16);
        for line in &[0, 1, 238, 239] {
            prepare_row(&mut p, *line);
            assert_eq!(p.sprites.active_count, 0, "Y={} line={}", y, line);
        }
    }
    put_sprite(&mut p, 0, 238, 1, 0, 16);
    prepare_row(&mut p, 239);
    assert_eq!(p.sprites.active_count, 1);
}

#[test]
fn an_empty_scanline_replaces_previous_sprite_rows() {
    let mut p = fixture();
    put_sprite(&mut p, 0, 10, 1, 0, 16);
    prepare_row(&mut p, 11);
    assert_eq!(p.sprites.active_count, 1);
    prepare_row(&mut p, 30);
    assert_eq!(p.sprites.active_count, 0);
    p.cycle = 17;
    assert!(p.sprite_pixel().is_none());
}

fn pixel_fixture(background: u8, attributes: u8, zero: bool, x: u8) -> Ppu {
    let mut p = fixture();
    p.scanline = 20;
    p.cycle = x as u16 + 1;
    p.tile_data = (background as u64) << 60;
    p.sprites.active_count = 1;
    p.sprites.active[0] = Sprite {
        x: x,
        attributes: attributes,
        low: 0xff,
        high: 0,
        zero: zero,
    };
    p
}

#[test]
fn priority_combines_opaque_and_transparent_backgrounds() {
    for background in &[0, 1, 4] {
        for attributes in &[0, 0x20] {
            let mut p = pixel_fixture(*background, *attributes, true, 16);
            p.step();
            let behind = *attributes == 0x20 && background & 3 != 0;
            assert_eq!(
                p.screen[20 * 256 + 16],
                SYSTEM_PALETTE[if behind { 1 } else { 0x16 }]
            );
            p.step();
            assert_eq!(p.status & STATUS_SPRITE_ZERO_HIT != 0, background & 3 != 0);
        }
    }
}

#[test]
fn earlier_behind_sprite_masks_later_foreground_sprite() {
    let mut p = pixel_fixture(1, 0x20, false, 16);
    p.sprites.active_count = 2;
    p.sprites.active[1] = Sprite {
        x: 16,
        attributes: 0,
        low: 0,
        high: 0xff,
        zero: false,
    };
    p.step();
    assert_eq!(p.screen[20 * 256 + 16], SYSTEM_PALETTE[1]);
    p = pixel_fixture(1, 0, true, 16);
    p.sprites.active_count = 2;
    p.sprites.active[0].low = 0;
    p.sprites.active[1] = Sprite {
        x: 16,
        attributes: 0,
        low: 0,
        high: 0xff,
        zero: false,
    };
    p.step();
    assert_eq!(p.screen[20 * 256 + 16], SYSTEM_PALETTE[0x1a]);
    p.step();
    assert_eq!(p.status & STATUS_SPRITE_ZERO_HIT, 0);
}

#[test]
fn layer_enables_and_left_clipping_gate_pixels_and_hits_independently() {
    for mask in 0..32u8 {
        for x in &[0, 7, 8, 16, 254, 255] {
            let mut p = pixel_fixture(1, 0, true, *x);
            p.mask = mask;
            let background = mask & 8 != 0 && (*x >= 8 || mask & 2 != 0);
            let sprite = mask & 16 != 0 && (*x >= 8 || mask & 4 != 0);
            p.step();
            let color = if sprite {
                0x16
            } else if background {
                1
            } else {
                0x0f
            };
            let output = if mask & 1 != 0 { color & 0x30 } else { color };
            assert_eq!(
                p.screen[20 * 256 + *x as usize],
                SYSTEM_PALETTE[output],
                "mask={:02x}, x={}",
                mask,
                x
            );
            p.step();
            assert_eq!(
                p.status & STATUS_SPRITE_ZERO_HIT != 0,
                background && sprite && *x != 255,
                "mask={:02x}, x={}",
                mask,
                x
            );
        }
    }
}

#[test]
fn right_edge_pixels_clip_without_wrapping_to_the_left() {
    let mut p = pixel_fixture(0, 0, false, 255);
    for x in &[0, 1, 254, 255] {
        p.cycle = *x as u16 + 1;
        assert_eq!(p.sprite_pixel().is_some(), *x == 255);
    }
    p.step();
    assert_eq!(p.screen[20 * 256 + 255], SYSTEM_PALETTE[0x16]);
}

#[test]
fn sprite_zero_hit_uses_pattern_opacity_not_rgb_and_has_dot_visibility() {
    let mut p = pixel_fixture(1, 0x20, true, 16);
    p.vram.write(0x3f01, 0x0f);
    p.vram.write(0x3f11, 0x0f);
    p.step();
    assert_eq!(p.status & STATUS_SPRITE_ZERO_HIT, 0);
    p.step();
    assert_ne!(p.status & STATUS_SPRITE_ZERO_HIT, 0);
    assert_ne!(p.read_status() & STATUS_SPRITE_ZERO_HIT, 0);
    assert_ne!(p.read_status() & STATUS_SPRITE_ZERO_HIT, 0);
    p.mask = 0;
    p.scanline = 241;
    p.cycle = 1;
    p.step();
    assert_ne!(p.status & STATUS_SPRITE_ZERO_HIT, 0);
    p.scanline = 261;
    p.cycle = 1;
    p.step();
    assert_eq!(
        p.status & (STATUS_SPRITE_ZERO_HIT | STATUS_SPRITE_OVERFLOW),
        0
    );
}

#[test]
fn only_sprite_zero_can_trigger_the_hit_flag() {
    let mut p = pixel_fixture(1, 0, false, 16);
    p.step();
    p.step();
    assert_eq!(p.status & STATUS_SPRITE_ZERO_HIT, 0);
}

#[test]
fn sprite_colors_use_existing_grayscale_and_emphasis_output() {
    let mut p = pixel_fixture(0, 0, false, 16);
    p.mask |= 1;
    p.vram.write(0x3f11, 0x21);
    p.step();
    assert_eq!(p.screen[20 * 256 + 16], SYSTEM_PALETTE[0x20]);
    p = pixel_fixture(0, 0, false, 16);
    p.mask |= 0x20;
    p.vram.write(0x3f11, 0x21);
    p.step();
    assert_eq!(p.screen[20 * 256 + 16], 0x3f8fbf);
}

#[test]
fn overflow_counts_nine_and_is_visible_on_the_evaluation_dot() {
    let mut p = fixture();
    for index in 0..9 {
        put_sprite(&mut p, index, 0, 1, 0, 16);
    }
    p.scanline = 0;
    p.cycle = 1;
    for _ in 1..130 {
        p.step();
    }
    assert_eq!(p.sprites.count, 8);
    assert_eq!(p.status & STATUS_SPRITE_OVERFLOW, 0);
    p.step();
    assert_ne!(p.status & STATUS_SPRITE_OVERFLOW, 0);
    p.read_status();
    assert_ne!(p.status & STATUS_SPRITE_OVERFLOW, 0);
}

#[test]
fn overflow_diagonal_scan_can_report_false_positives_and_false_negatives() {
    let mut p = fixture();
    for index in 0..8 {
        put_sprite(&mut p, index, 32, 1, 0, 16);
    }
    put_sprite(&mut p, 9, 255, 32, 0xff, 255); // tile byte acts as Y
    prepare_row(&mut p, 33);
    assert_ne!(p.status & STATUS_SPRITE_OVERFLOW, 0);
    p = fixture();
    for index in 0..8 {
        put_sprite(&mut p, index, 32, 1, 0, 16);
    }
    put_sprite(&mut p, 9, 32, 255, 0xff, 255); // actual ninth Y is skipped
    prepare_row(&mut p, 33);
    assert_eq!(p.status & STATUS_SPRITE_OVERFLOW, 0);
}

#[test]
fn overflow_and_evaluation_depend_on_either_layer_being_enabled() {
    for mask in &[0, 8, 16, 24] {
        let mut p = fixture();
        p.mask = *mask;
        for index in 0..9 {
            put_sprite(&mut p, index, 32, 1, 0, 0);
        }
        prepare_row(&mut p, 33);
        assert_eq!(p.status & STATUS_SPRITE_OVERFLOW != 0, *mask != 0);
        assert_eq!(p.sprites.active_count, if *mask == 0 { 0 } else { 8 });
        p.mask = 0;
        p.scanline = 261;
        p.cycle = 1;
        p.step();
        assert_eq!(p.status & STATUS_SPRITE_OVERFLOW, 0);
    }
}

#[test]
fn evaluation_uses_current_sprite_height_and_fetches_pattern_planes_at_their_dots() {
    let mut p = fixture();
    put_sprite(&mut p, 0, 16, 1, 0, 16);
    prepare_row(&mut p, 29);
    assert_eq!(p.sprites.active_count, 0);
    p.ctrl = 0x20;
    prepare_row(&mut p, 29);
    assert_eq!(p.sprites.active_count, 1);
    p = fixture();
    put_sprite(&mut p, 0, 10, 1, 0, 16);
    p.scanline = 10;
    p.cycle = 1;
    for _ in 1..261 {
        p.step();
    }
    p.vram.write(16, 0x80);
    p.step(); // dot 261 fetches low plane
    p.vram.write(16, 0);
    p.vram.write(24, 0x40);
    p.step();
    p.step(); // dot 263 fetches high plane
    p.vram.write(24, 0);
    assert_eq!(
        (p.sprites.active[0].low, p.sprites.active[0].high),
        (0x80, 0x40)
    );
    assert_eq!(p.spr_ram_addr, 0);
}

#[test]
fn oam_ports_mask_attribute_bits_and_preserve_read_address() {
    let mut p = fixture();
    p.mask = 0;
    for addr in 0..256u16 {
        p.write_register(3, addr as u8);
        p.write_register(4, 0xff);
        p.write_register(3, addr as u8);
        let expected = if addr % 4 == 2 { 0xe3 } else { 0xff };
        assert_eq!(p.read_register(4), expected);
        assert_eq!(p.read_register(4), expected);
        assert_eq!(p.spr_ram_addr, addr as u8);
    }
}

#[test]
fn real_cpu_dma_and_ppu_clock_render_a_sprite() {
    use cpu::Cpu;
    use interconnect::{Interconnect, MemoryMappingInterconnect};
    use rom::{Mirroring, Rom};
    let mut prg = vec![0xea; 16384];
    prg[..13].copy_from_slice(&[
        0xa9, 2, 0x8d, 0x14, 0x40, 0xa9, 0x14, 0x8d, 1, 0x20, 0x4c, 0x0a, 0xc0,
    ]);
    prg[0x3ffc] = 0;
    prg[0x3ffd] = 0xc0;
    let rom = Rom {
        prg_rom: vec![prg],
        chr_rom: vec![],
        mapper: 2, submapper: 0,
        mirroring: Mirroring::Horizontal,
    };
    let mut bus = MemoryMappingInterconnect::new(rom);
    for addr in 0x200..0x300 {
        bus.write_word(addr, 0xff);
    }
    for (offset, value) in [19, 1, 0, 50].iter().enumerate() {
        bus.write_word(0x200 + offset as u16, *value);
    }
    bus.ppu.vram.write(0x3f00, 0x0f);
    bus.ppu.vram.write(0x3f11, 0x16);
    for row in 0..8 {
        bus.ppu.vram.write(16 + row, 0xff);
    }
    let mut cpu = Cpu::new();
    cpu.reset(&mut bus);
    for _ in 0..10000 {
        cpu.step(&mut bus);
        if bus.ppu.screen[20 * 256 + 50] == SYSTEM_PALETTE[0x16] {
            return;
        }
    }
    panic!("DMA-loaded sprite did not reach the framebuffer");
}

fn frame(p: &mut Ppu) {
    for _ in 0..100000 {
        if p.step().end_frame {
            return;
        }
    }
    panic!("no frame boundary");
}

#[test]
fn combined_background_and_sprite_frames_match_reference_composition() {
    for tall in &[false, true] {
        for bank in &[0, 8] {
            for mask in &[0x18, 0x1e] {
                let mut p = fixture();
                load_patterns(&mut p);
                let mut palette = [0u8; 32];
                for index in 0..32usize {
                    palette[index] = if index % 4 == 0 {
                        0x0f
                    } else {
                        ((index * 3 + 5) % 64) as u8
                    };
                    p.vram.write(0x3f00 + index as u16, palette[index]);
                }
                for y in 0..30u16 {
                    for x in 0..32u16 {
                        p.vram
                            .write(0x2000 + y * 32 + x, ((y * 7 + x * 3) % 256) as u8);
                    }
                }
                for index in 0..64u8 {
                    let y = if index < 12 {
                        32
                    } else {
                        ((index as u16 * 17) % 238) as u8
                    };
                    let x = if index < 12 {
                        24 + index * 5
                    } else {
                        (index as u16 * 37) as u8
                    };
                    let attr = (index & 3)
                        | if index & 4 != 0 { 0x20 } else { 0 }
                        | if index & 8 != 0 { 0x40 } else { 0 }
                        | if index & 16 != 0 { 0x80 } else { 0 };
                    put_sprite(
                        &mut p,
                        index,
                        y,
                        index.wrapping_mul(29).wrapping_add(3),
                        attr,
                        x,
                    );
                }
                p.spr_ram[60 * 4] = 0;
                p.spr_ram[61 * 4] = 238;
                p.spr_ram[62 * 4] = 239;
                p.spr_ram[63 * 4] = 255;
                let oam = p.spr_ram;
                p.write_ctrl(if *tall { 0x20 | bank } else { *bank });
                p.write_mask(*mask);
                let height = if *tall { 16 } else { 8 };
                for frame_number in 0..2 {
                    frame(&mut p);
                    for y in 0..240usize {
                        // Reference chooses the first eight by Y, independently of
                        // the emulated evaluation counters and fetched row cache.
                        let selected: Vec<_> = (0..64usize)
                            .filter(|i| {
                                y > oam[i * 4] as usize && y <= oam[i * 4] as usize + height
                            })
                            .take(8)
                            .collect();
                        for x in 0..256usize {
                            let tile = (((y / 8) * 7 + (x / 8) * 3) % 256) as u8;
                            let background = if x < 8 && mask & 2 == 0 {
                                0
                            } else {
                                pattern(tile, 0, (y % 8) as u8, (x % 8) as u8)
                            };
                            let mut entry = if background == 0 {
                                0
                            } else {
                                background as usize
                            };
                            if x >= 8 || mask & 4 != 0 {
                                for index in &selected {
                                    let sprite = &oam[index * 4..index * 4 + 4];
                                    let sx = sprite[3] as usize;
                                    if x < sx || x >= sx + 8 {
                                        continue;
                                    }
                                    let mut row = y - sprite[0] as usize - 1;
                                    let mut column = x - sx;
                                    if sprite[2] & 0x80 != 0 {
                                        row = height - 1 - row;
                                    }
                                    if sprite[2] & 0x40 != 0 {
                                        column = 7 - column;
                                    }
                                    let source_tile = if *tall {
                                        (sprite[1] & 0xfe) + (row / 8) as u8
                                    } else {
                                        sprite[1]
                                    };
                                    let source_bank = if *tall { sprite[1] & 1 } else { bank >> 3 };
                                    let color = pattern(
                                        source_tile,
                                        source_bank,
                                        (row % 8) as u8,
                                        column as u8,
                                    );
                                    if color == 0 {
                                        continue;
                                    }
                                    if background == 0 || sprite[2] & 0x20 == 0 {
                                        entry =
                                            0x10 | ((sprite[2] as usize & 3) << 2) | color as usize;
                                    }
                                    break;
                                }
                            }
                            let expected = SYSTEM_PALETTE[palette[entry] as usize];
                            assert_eq!(
                                p.screen[y * 256 + x],
                                expected,
                                "tall={} bank={} mask={:02x} frame={} pixel=({}, {})",
                                tall,
                                bank,
                                mask,
                                frame_number,
                                x,
                                y
                            );
                        }
                    }
                }
            }
        }
    }
}
