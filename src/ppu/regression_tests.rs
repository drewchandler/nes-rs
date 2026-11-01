use super::*;

fn pixel_ppu() -> Ppu {
    let mut p = Ppu::new();
    p.mask = 0x0a;
    p.scanline = 0;
    p.cycle = 9;
    p.vram.write(0x3f00, 1);
    p.vram.write(0x3f01, 2);
    p.tile_data = 1u64 << 60;
    p
}

#[test]
fn ctrl_bit_two_increments_vram_by_32() {
    let mut p = Ppu::new();
    p.write_ctrl(0x04);
    p.vram_addr = 0x2100;
    p.write_vram_data(0xab);
    assert_eq!(p.vram_addr, 0x2120);
}

#[test]
fn ctrl_bit_one_does_not_change_vram_increment() {
    let mut p = Ppu::new();
    p.write_ctrl(0x02);
    p.vram_addr = 0x2100;
    p.write_vram_data(0xab);
    assert_eq!(p.vram_addr, 0x2101);
}

#[test]
fn fine_scroll_extracts_only_one_pixel_nibble() {
    let mut p = pixel_ppu();
    p.scroll = 1;
    p.tile_data = 0x1200000000000000;
    p.vram.write(0x3f02, 1);
    p.vram.write(0x3f12, 2);
    p.step();
    assert_eq!(p.screen[8], SYSTEM_PALETTE[1]);
}

#[test]
fn transparent_background_uses_universal_backdrop() {
    let mut p = pixel_ppu();
    p.tile_data = 4u64 << 60;
    p.vram.write(0x3f04, 2);
    p.step();
    assert_eq!(p.screen[8], SYSTEM_PALETTE[1]);
}

#[test]
fn special_palette_entries_alias_background_entries() {
    let mut p = Ppu::new();
    for offset in &[0, 4, 8, 12] {
        p.vram.write(0x3f00 + offset, 0x21);
        assert_eq!(p.vram.read(0x3f10 + offset), 0x21, "offset {}", offset);
    }
}

#[test]
fn default_horizontal_nametable_mirroring() {
    let mut p = Ppu::new();
    p.vram.write(0x2000, 0xab);
    assert_eq!(p.vram.read(0x2400), 0xab);
    assert_eq!(p.vram.read(0x2800), 0);
}

#[test]
fn prerender_reloads_horizontal_scroll_at_dot_257() {
    let mut p = Ppu::new();
    p.mask = 0x0a;
    p.scanline = 261;
    p.cycle = 257;
    p.vram_addr = 0x041f;
    p.tmp_vram_addr = 5;
    p.step();
    assert_eq!(p.vram_addr & 0x041f, 5);
}

#[test]
fn vertical_scroll_increments_at_dot_256() {
    let mut p = Ppu::new();
    p.mask = 0x0a;
    p.scanline = 0;
    p.cycle = 256;
    p.step();
    assert_eq!(p.vram_addr & 0x7000, 0x1000);
}

#[test]
fn palette_reads_are_immediate() {
    let mut p = Ppu::new();
    p.buffered_read = 0x55;
    p.vram_addr = 0x3f00;
    p.vram.write(0x3f00, 0x21);
    assert_eq!(p.read_vram_data(), 0x21);
}

#[test]
fn palette_reads_refill_buffer_from_underlying_nametable() {
    let mut p = Ppu::new();
    p.vram_addr = 0x3f00;
    p.vram.write(0x3f00, 0x21);
    p.vram.write(0x2f00, 0x35);
    p.read_vram_data();
    p.vram_addr = 0x2000;
    assert_eq!(p.read_vram_data(), 0x35);
}

#[test]
fn blanking_draws_backdrop_instead_of_leaving_previous_frame() {
    let mut p = pixel_ppu();
    p.mask = 0;
    p.screen[8] = SYSTEM_PALETTE[2];
    p.step();
    assert_eq!(p.screen[8], SYSTEM_PALETTE[1]);
}

#[test]
fn sprite_only_mask_does_not_draw_background() {
    let mut p = pixel_ppu();
    p.mask = 0x10;
    p.step();
    assert_eq!(p.screen[8], SYSTEM_PALETTE[1]);
}

#[test]
fn left_eight_background_pixels_are_clipped_unless_enabled() {
    let mut p = pixel_ppu();
    p.mask = 0x08;
    p.cycle = 1;
    p.step();
    assert_eq!(p.screen[0], SYSTEM_PALETTE[1]);
}

#[test]
fn grayscale_mask_removes_palette_chroma() {
    let mut p = pixel_ppu();
    p.mask = 0x0b;
    p.vram.write(0x3f01, 0x21);
    p.step();
    assert_eq!(p.screen[8], SYSTEM_PALETTE[0x20]);
}

#[test]
fn emphasis_bits_change_rendered_rgb() {
    let mut p = pixel_ppu();
    p.vram.write(0x3f01, 0x21);
    p.step();
    let normal = p.screen[8];
    p.cycle = 9;
    p.tile_data = 1u64 << 60;
    p.mask = 0x2a;
    p.step();
    assert_ne!(p.screen[8], normal);
}

#[test]
fn enabling_nmi_during_vblank_emits_an_edge() {
    let mut p = Ppu::new();
    p.scanline = 241;
    p.cycle = 10;
    p.set_vblank(true);
    p.write_ctrl(0x80);
    assert!(p.step().nmi);
}

fn frame_length(p: &mut Ppu) -> usize {
    for n in 1..100000 {
        if p.step().end_frame {
            return n;
        }
    }
    panic!("no frame")
}

#[test]
fn rendering_skips_one_dot_on_alternate_ntsc_frames() {
    let mut p = Ppu::new();
    p.mask = 0x0a;
    frame_length(&mut p);
    let a = frame_length(&mut p);
    let b = frame_length(&mut p);
    assert_eq!(
        (::std::cmp::min(a, b), ::std::cmp::max(a, b)),
        (89341, 89342)
    );
}

#[test]
fn vram_address_increment_wraps_at_15_bits() {
    let mut p = Ppu::new();
    p.vram_addr = 0x7fff;
    p.write_vram_data(0);
    assert_eq!(p.vram_addr, 0);
}

#[test]
fn data_access_during_rendering_increments_both_scroll_axes() {
    let mut p = Ppu::new();
    p.mask = 0x0a;
    p.scanline = 0;
    p.cycle = 100;
    p.write_vram_data(0);
    assert_eq!(p.vram_addr, 0x1001);
}

#[test]
fn background_pipeline_bitplanes_and_attributes_exhaustive() {
    let mut p = Ppu::new();
    for low in 0..256u16 {
        for high in 0..256u16 {
            for attribute in 0..4u8 {
                p.low_bg_tile_byte = low as u8;
                p.high_bg_tile_byte = high as u8;
                p.attribute_table_byte = attribute << 2;
                p.tile_data = 0;
                p.push_tile_data();
                let mut expected = 0u64;
                for pixel in 0..8 {
                    let bit = 7 - pixel;
                    expected = (expected << 4)
                        | ((attribute as u64) << 2)
                        | (((high as u64 >> bit) & 1) << 1)
                        | ((low as u64 >> bit) & 1);
                }
                assert_eq!(p.tile_data, expected);
            }
        }
    }
}

#[test]
fn attribute_fetch_selects_correct_quadrant() {
    let mut p = Ppu::new();
    for nt in 0..4u16 {
        for y in 0..30u16 {
            for x in 0..32u16 {
                p.vram_addr = (nt << 10) | (y << 5) | x;
                let addr = 0x23c0 + (nt << 10) + (y / 4) * 8 + x / 4;
                p.vram.write(addr, 0b11100100);
                p.fetch_attribute_table_byte();
                let quadrant = ((y % 4) / 2) * 2 + (x % 4) / 2;
                assert_eq!(p.attribute_table_byte, (quadrant as u8) << 2);
            }
        }
    }
}

#[test]
fn scroll_register_encodes_all_xy_offsets_correctly() {
    let mut p = Ppu::new();
    for x in 0..256u16 {
        for y in 0..256u16 {
            p.tmp_vram_addr = 0x0c00;
            p.write_flag = false;
            p.write_scroll(x as u8);
            p.write_scroll(y as u8);
            assert_eq!(
                p.tmp_vram_addr,
                0x0c00 | (x >> 3) | ((y >> 3) << 5) | ((y & 7) << 12)
            );
            assert_eq!(p.scroll, (x & 7) as u8);
        }
    }
}

#[test]
fn vertical_reload_preserves_horizontal_fields() {
    let mut p = Ppu::new();
    p.mask = 0x0a;
    p.scanline = 261;
    p.cycle = 280;
    p.vram_addr = 0x0403;
    p.tmp_vram_addr = 0x7be0;
    p.step();
    assert_eq!(p.vram_addr, 0x7fe3);
}

#[test]
fn ordinary_vram_reads_are_buffered() {
    let mut p = Ppu::new();
    p.buffered_read = 0x55;
    p.vram_addr = 0x2000;
    p.vram.write(0x2000, 0xab);
    assert_eq!(p.read_vram_data(), 0x55);
    assert_eq!(p.read_vram_data(), 0xab);
}

#[test]
fn basic_vram_and_32_byte_palette_mirrors_work() {
    let mut p = Ppu::new();
    p.vram.write(0x2000, 0xab);
    assert_eq!(p.vram.read(0x3000), 0xab);
    p.vram.write(0x3f01, 0x21);
    assert_eq!(p.vram.read(0x3f21), 0x21);
    assert_eq!(p.vram.read(0x7f21), 0x21);
}

#[test]
fn blanked_frames_have_constant_dot_count() {
    let mut p = Ppu::new();
    frame_length(&mut p);
    assert_eq!(frame_length(&mut p), 89342);
    assert_eq!(frame_length(&mut p), 89342);
}

#[test]
fn status_read_clears_vblank_and_shared_write_latch() {
    let mut p = Ppu::new();
    p.set_vblank(true);
    p.write_scroll(1);
    assert_eq!(p.read_status() & 0x80, 0x80);
    assert!(!p.write_flag);
    assert_eq!(p.read_status() & 0x80, 0);
}

fn fixture() -> Ppu {
    let mut p = Ppu::new();
    p.mask = 0x0a;
    for tile in 0..256u16 {
        for row in 0..8u16 {
            let mut low = 0;
            let mut high = 0;
            for x in 0..8u16 {
                let color = (tile + row + x) % 3 + 1;
                low = (low << 1) | (color as u8 & 1);
                high = (high << 1) | ((color as u8 >> 1) & 1);
            }
            p.vram.write(tile * 16 + row, low);
            p.vram.write(tile * 16 + row + 8, high);
        }
    }
    // Duplicate the layout in all nametables to isolate the raster pipeline
    // from the separate, confirmed cartridge-mirroring defect.
    for nt in 0..4u16 {
        for y in 0..30u16 {
            for x in 0..32u16 {
                p.vram
                    .write(0x2000 + (nt << 10) + y * 32 + x, (y * 32 + x) as u8);
            }
        }
    }
    p.vram.write(0x3f00, 0x0f);
    p.vram.write(0x3f01, 1);
    p.vram.write(0x3f02, 0x16);
    p.vram.write(0x3f03, 0x20);
    p
}

fn frame(p: &mut Ppu) {
    for _ in 0..100000 {
        if p.step().end_frame {
            return;
        }
    }
    panic!("no frame")
}

fn assert_frame(p: &Ppu) {
    let colors = [0, 1, 0x16, 0x20];
    let mut mismatches = 0;
    let mut first = None;
    let mut rows = vec![0usize; 240];
    for y in 0..240usize {
        for x in 0..256usize {
            let tile = ((y / 8) * 32 + x / 8) % 256;
            let color = (tile + y % 8 + x % 8) % 3 + 1;
            let expected = SYSTEM_PALETTE[colors[color]];
            let actual = p.screen[y * 256 + x];
            if actual != expected {
                mismatches += 1;
                rows[y] += 1;
                if first.is_none() {
                    first = Some((x, y, actual, expected));
                }
            }
        }
    }
    let affected: Vec<_> = rows.iter().enumerate().filter(|&(_, n)| *n != 0).collect();
    assert_eq!(
        mismatches, 0,
        "first mismatched pixel {:?}, affected rows {:?}",
        first, affected
    );
}

#[test]
fn first_static_background_frame_matches_reference_raster() {
    let mut p = fixture();
    frame(&mut p);
    assert_frame(&p);
}

#[test]
fn steady_static_background_frame_matches_reference_raster() {
    let mut p = fixture();
    frame(&mut p);
    frame(&mut p);
    assert_frame(&p);
}

#[test]
fn cartridge_mirroring_routes_all_logical_tables_and_mirrors() {
    for &(mirroring, expected) in &[
        (Mirroring::Horizontal, [11, 11, 13, 13]),
        (Mirroring::Vertical, [12, 13, 12, 13]),
        (Mirroring::FourScreen, [10, 11, 12, 13]),
    ] {
        let mut vram = Vram::with_mirroring(mirroring);
        for table in 0..4u16 {
            for offset in &[0, 0x123, 0x3ff] {
                vram.write(0x2000 + table * 0x400 + offset, 10 + table as u8);
            }
        }
        for table in 0..4u16 {
            for offset in &[0, 0x123, 0x3ff] {
                let addr = 0x2000 + table * 0x400 + offset;
                assert_eq!(vram.read(addr), expected[table as usize]);
                assert_eq!(vram.read(addr + 0x4000), expected[table as usize]);
                if addr <= 0x2eff {
                    assert_eq!(vram.read(addr + 0x1000), expected[table as usize]);
                }
            }
        }
    }
}

#[test]
fn palette_aliases_are_bidirectional_six_bit_and_repeat_every_32_bytes() {
    let mut vram = Vram::new();
    for index in &[0, 4, 8, 12] {
        vram.write(0x3f10 + index, 0xff);
        assert_eq!(vram.read(0x3f00 + index), 0x3f);
        vram.write(0x3f00 + index, 0x21);
        for repeat in 0..8u16 {
            assert_eq!(vram.read(0x3f10 + index + repeat * 32), 0x21);
            assert_eq!(vram.read(0x7f10 + index + repeat * 32), 0x21);
        }
    }
    vram.write(0x3f01, 1);
    vram.write(0x3f11, 2);
    assert_eq!((vram.read(0x3f01), vram.read(0x3f11)), (1, 2));
}

#[test]
fn cpu_palette_reads_keep_latch_upper_bits_and_refill_the_buffer() {
    let mut p = Ppu::new();
    p.vram.write(0x3f00, 0xff);
    p.vram.write(0x2f00, 0xab);
    p.write_register(0x2006, 0x3f);
    p.write_register(0x2006, 0);
    p.write_register(0x2001, 0x81); // Grayscale, latch upper bits $80.
    assert_eq!(p.read_register(0x2007), 0xb0);
    assert_eq!(p.read_register(0x2000), 0xb0);
    p.write_register(0x2006, 0x20);
    p.write_register(0x2006, 0);
    assert_eq!(p.read_register(0x2007), 0xab);
    // Writing even a read-only register updates the PPU latch.
    p.write_register(0x2002, 0xc5);
    assert_eq!(p.read_register(0x2001), 0xc5);
    assert_eq!(p.read_register(0x2002), 5);
}

#[test]
fn blanking_can_output_the_addressed_palette_entry() {
    let mut p = pixel_ppu();
    p.mask = 0;
    p.vram_addr = 0x3f05;
    p.vram.write(0x3f05, 0x21);
    p.step();
    assert_eq!(p.screen[8], SYSTEM_PALETTE[0x21]);
    p.cycle = 9;
    p.vram_addr = 0x2000;
    p.step();
    assert_eq!(p.screen[8], SYSTEM_PALETTE[1]);
}

#[test]
fn left_clipping_stops_at_pixel_eight_and_respects_enable_bit() {
    for &(dot, mask, visible) in &[(1, 8, false), (8, 8, false), (9, 8, true), (1, 10, true)] {
        let mut p = pixel_ppu();
        p.cycle = dot;
        p.mask = mask;
        p.step();
        assert_eq!(
            p.screen[dot as usize - 1],
            SYSTEM_PALETTE[if visible { 2 } else { 1 }]
        );
    }
}

#[test]
fn nmi_is_edge_triggered_and_not_repeated_while_high() {
    let mut p = Ppu::new();
    p.scanline = 241;
    p.cycle = 10;
    p.set_vblank(true);
    assert!(!p.step().nmi);
    p.write_register(0, 0x80);
    assert!(p.step().nmi);
    assert!(!p.step().nmi);
    p.write_register(0, 0x80);
    assert!(!p.step().nmi);
    p.write_register(0, 0);
    p.write_register(0, 0x80);
    assert!(p.step().nmi);
    p.read_register(2);
    assert!(!p.nmi_output);
    p.write_register(0, 0);
    p.write_register(0, 0x80);
    assert!(!p.step().nmi);
    p.set_vblank(true);
    assert!(p.step().nmi);
}

#[test]
fn vblank_transitions_generate_one_nmi_and_clear_on_prerender() {
    let mut p = Ppu::new();
    p.write_ctrl(0x80);
    p.scanline = 241;
    p.cycle = 0;
    assert!(!p.step().nmi);
    assert!(p.step().nmi);
    assert!(!p.step().nmi);
    assert_eq!(p.status & 0x80, 0x80);
    p.scanline = 261;
    p.cycle = 1;
    assert!(!p.step().nmi);
    assert_eq!(p.status & 0x80, 0);
    assert!(!p.nmi_output);
}

#[test]
fn odd_frame_skip_depends_on_rendering_at_the_skip_dot() {
    for mask in &[0, 8, 16] {
        let mut p = Ppu::new();
        p.mask = *mask;
        p.odd_frame = true;
        p.scanline = 261;
        p.cycle = 339;
        assert_eq!(p.step().end_frame, *mask != 0);
        if *mask == 0 {
            assert_eq!(p.cycle, 340);
            assert!(p.step().end_frame);
        }
        assert_eq!((p.scanline, p.cycle, p.odd_frame), (0, 0, false));
    }
}

#[test]
fn data_port_uses_scroll_increments_only_on_render_scanlines() {
    for scanline in &[-1, 0, 239, 240, 241, 261] {
        for mask in &[0, 8, 16] {
            for read in &[false, true] {
                let mut p = Ppu::new();
                p.scanline = *scanline;
                p.cycle = 100;
                p.mask = *mask;
                p.ctrl = 4;
                p.vram_addr = 0x73bf; // fine Y=7, coarse Y=29, coarse X=31
                if *read {
                    p.read_vram_data();
                } else {
                    p.write_vram_data(0);
                }
                let rendering = *mask != 0 && (*scanline < 240 || *scanline == 261);
                assert_eq!(p.vram_addr, if rendering { 0x0c00 } else { 0x73df });
            }
        }
    }
    let mut p = Ppu::new();
    p.ctrl = 4;
    p.vram_addr = 0x7ff0;
    p.write_vram_data(0);
    assert_eq!(p.vram_addr, 0x10);
}

#[test]
fn all_emphasis_combinations_preserve_or_attenuate_rgb_channels() {
    let mut p = Ppu::new();
    let original = SYSTEM_PALETTE[0x21];
    for emphasis in 0..8u8 {
        p.mask = emphasis << 5;
        let actual = p.palette_color(0x21);
        for &(shift, other_channels) in &[(16, 6), (8, 5), (0, 3)] {
            let before = (original >> shift) & 0xff;
            let after = (actual >> shift) & 0xff;
            if emphasis & other_channels == 0 {
                assert_eq!(after, before);
            } else {
                assert!(after < before);
            }
        }
    }
}

fn patterned_background(mirroring: Mirroring) -> Ppu {
    let mut p = Ppu::with_mirroring(mirroring);
    for tile in 0..256u16 {
        for row in 0..8u16 {
            let (mut low, mut high) = (0, 0);
            for x in 0..8u16 {
                let color = (tile + row + x) % 4;
                low = (low << 1) | (color as u8 & 1);
                high = (high << 1) | ((color as u8 >> 1) & 1);
            }
            p.vram.write(tile * 16 + row, low);
            p.vram.write(tile * 16 + row + 8, high);
        }
    }
    let physical_tables = if mirroring == Mirroring::FourScreen {
        4
    } else {
        2
    };
    for physical in 0..physical_tables {
        let logical = if mirroring == Mirroring::Horizontal {
            physical * 2
        } else {
            physical
        };
        let base = 0x2000 + logical * 0x400;
        for y in 0..30u16 {
            for x in 0..32u16 {
                p.vram.write(
                    base + y * 32 + x,
                    ((physical * 53 + y * 7 + x * 13) % 256) as u8,
                );
            }
        }
        for block_y in 0..8u16 {
            for block_x in 0..8u16 {
                let mut attribute = 0;
                for quadrant_y in 0..2u16 {
                    for quadrant_x in 0..2u16 {
                        let palette =
                            (block_x * 2 + quadrant_x + block_y * 2 + quadrant_y + physical) % 4;
                        attribute |= (palette as u8) << ((quadrant_y * 2 + quadrant_x) * 2);
                    }
                }
                p.vram
                    .write(base + 0x3c0 + block_y * 8 + block_x, attribute);
            }
        }
    }
    for (index, color) in RASTER_PALETTE.iter().enumerate() {
        p.vram.write(0x3f00 + index as u16, *color);
    }
    p
}

const RASTER_PALETTE: [u8; 16] = [
    0x0f, 0x01, 0x16, 0x20, 0x04, 0x06, 0x1a, 0x21, 0x08, 0x09, 0x12, 0x27, 0x0c, 0x15, 0x25, 0x31,
];

#[test]
fn scrolled_multiframe_rasters_match_independent_reference() {
    for mirroring in &[
        Mirroring::Horizontal,
        Mirroring::Vertical,
        Mirroring::FourScreen,
    ] {
        for nt in 0..4u16 {
            for &(scroll_x, scroll_y) in &[(0, 0), (1, 0), (7, 7), (8, 8), (255, 231), (253, 239)] {
                let mut p = patterned_background(*mirroring);
                p.write_register(0, nt as u8);
                p.write_register(5, scroll_x as u8);
                p.write_register(5, scroll_y as u8);
                p.write_register(1, 0x0a);
                for frame_number in 0..2 {
                    frame(&mut p);
                    for y in 0..240u16 {
                        for x in 0..256u16 {
                            let world_x = scroll_x + x;
                            let world_y = scroll_y + y;
                            let table =
                                ((nt & 1) ^ (world_x / 256)) | (((nt >> 1) ^ (world_y / 240)) << 1);
                            let physical = match *mirroring {
                                Mirroring::Horizontal => table / 2,
                                Mirroring::Vertical => table % 2,
                                Mirroring::FourScreen => table,
                                Mirroring::SingleLower => 0,
                                Mirroring::SingleUpper => 1,
                            };
                            let tile_x = (world_x % 256) / 8;
                            let tile_y = (world_y % 240) / 8;
                            let tile = (physical * 53 + tile_y * 7 + tile_x * 13) % 256;
                            let color = (tile + world_y % 8 + world_x % 8) % 4;
                            let palette = (tile_x / 2 + tile_y / 2 + physical) % 4;
                            let entry = if color == 0 { 0 } else { palette * 4 + color };
                            let expected = SYSTEM_PALETTE[RASTER_PALETTE[entry as usize] as usize];
                            assert_eq!(
                                p.screen[(y * 256 + x) as usize],
                                expected,
                                "{:?} nt={} scroll=({}, {}) frame={} pixel=({}, {})",
                                mirroring,
                                nt,
                                scroll_x,
                                scroll_y,
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

#[test]
fn background_pattern_table_one_uses_both_bitplanes_and_fine_y() {
    let mut p = fixture();
    for addr in 0..0x1000 {
        let byte = p.vram.read(addr);
        p.vram.write(addr + 0x1000, byte);
        p.vram.write(addr, 0);
    }
    p.write_ctrl(0x10);
    frame(&mut p);
    assert_frame(&p);
    frame(&mut p);
    assert_frame(&p);
}
