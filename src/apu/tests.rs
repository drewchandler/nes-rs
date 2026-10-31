use super::channels::{Envelope, Noise, Pulse, Triangle};
use super::dmc::{Dmc, PERIODS};
use super::mixer::{mix, Mixer};
use super::*;
use interconnect::{Interconnect, MemoryMappingInterconnect};
use rom::{Mirroring, Rom};

fn clock(apu: &mut Apu, cycles: usize) {
    for _ in 0..cycles {
        apu.step();
    }
}

fn bus(program: &[u8]) -> MemoryMappingInterconnect {
    let mut prg = vec![0xea; 16384];
    prg[..program.len()].copy_from_slice(program);
    prg[0x3ffc..0x4000].copy_from_slice(&[0, 0xc0, 0, 0xc1]);
    // IRQ handler increments RAM without relying on a test-only CPU accessor.
    prg[0x100..0x103].copy_from_slice(&[0xe6, 0, 0x40]);
    MemoryMappingInterconnect::new(Rom {
        prg_rom: vec![prg],
        chr_rom: vec![],
        mapper: 2, submapper: 0,
        mirroring: Mirroring::Horizontal,
    })
}

#[test]
fn powered_up_channels_are_inactive() {
    let mut apu = Apu::new();
    assert_eq!(apu.read_status(0), 0);
    for _ in 0..1000 {
        apu.step();
    }
    assert_eq!(apu.read_status(0), 0);
}

#[test]
fn all_length_entries_and_channel_enable_gates() {
    let expected = [
        10, 254, 20, 2, 40, 4, 80, 6, 160, 8, 60, 10, 14, 12, 26, 14, 12, 16, 24, 18, 48, 20, 96,
        22, 192, 24, 72, 26, 16, 28, 32, 30,
    ];
    for index in 0..32 {
        for &(register, bit) in &[(0x4003, 1), (0x4007, 2), (0x400b, 4), (0x400f, 8)] {
            let mut apu = Apu::new();
            apu.write_register(register, (index << 3) as u8);
            apu.step();
            assert_eq!(apu.read_status(0), 0, "disabled channel reloaded");
            apu.write_register(0x4015, bit);
            apu.write_register(register, (index << 3) as u8);
            apu.step();
            let length = match bit {
                1 => apu.pulse1.length.value,
                2 => apu.pulse2.length.value,
                4 => apu.triangle.length.value,
                _ => apu.noise.length.value,
            };
            assert_eq!(length, expected[index]);
            assert_eq!(apu.read_status(0) & 15, bit);
            apu.write_register(0x4015, 0);
            apu.write_register(0x4015, bit);
            assert_eq!(apu.read_status(0) & 15, 0, "enable must not reload length");
        }
    }
}

#[test]
fn four_step_clocks_and_three_irq_assertions() {
    let mut apu = Apu::new();
    apu.write_register(0x4015, 1);
    apu.write_register(0x4000, 0);
    apu.write_register(0x4003, 0x18); // Length 2.
    clock(&mut apu, 7456);
    assert_eq!(apu.pulse1.envelope.decay, 0);
    clock(&mut apu, 1);
    assert_eq!(apu.pulse1.envelope.decay, 15);
    clock(&mut apu, 14912 - 7457);
    assert_eq!(apu.pulse1.length.value, 2);
    clock(&mut apu, 1);
    assert_eq!(apu.pulse1.length.value, 1);
    clock(&mut apu, 29827 - 14913);
    assert!(!apu.irq());
    for _ in 0..3 {
        apu.step();
        assert_eq!(apu.read_status(0) & 0x40, 0x40);
        assert!(!apu.irq());
    }
    assert_eq!(apu.pulse1.length.value, 0);
    apu.step();
    assert_eq!(apu.read_status(0) & 0x40, 0);
}

#[test]
fn frame_counter_reset_waits_for_three_or_four_cycles() {
    for parity in 0..2 {
        let mut apu = Apu::new();
        clock(&mut apu, parity);
        apu.write_register(0x4015, 1);
        apu.write_register(0x4003, 0x18);
        apu.write_register(0x4017, 0x80);
        let delay = if parity == 0 { 3 } else { 4 };
        clock(&mut apu, delay - 1);
        assert_eq!(apu.pulse1.length.value, 2);
        apu.step();
        assert_eq!(apu.pulse1.length.value, 1);
        assert_eq!(apu.frame_cycle, 0);
        clock(&mut apu, 14912);
        assert_eq!(apu.pulse1.length.value, 1);
        apu.step();
        assert_eq!(apu.pulse1.length.value, 0);
        clock(&mut apu, 37282 * 2);
        assert!(!apu.irq(), "five-step mode must not assert frame IRQ");
    }
}

#[test]
fn status_open_bus_and_independent_irq_clear_rules() {
    let mut apu = Apu::new();
    apu.frame_irq = true;
    apu.dmc.irq = true;
    assert_eq!(apu.read_status(0xff), 0xe0);
    assert_eq!(apu.read_status(0), 0x80);
    apu.write_register(0x4015, 0);
    assert!(!apu.irq());
    for value in [0, 0x80] {
        apu.frame_irq = true;
        apu.write_register(0x4017, value);
        assert!(apu.frame_irq);
    }
    for value in [0x40, 0xc0] {
        apu.frame_irq = true;
        apu.dmc.irq = true;
        apu.write_register(0x4017, value);
        assert!(!apu.frame_irq);
        assert!(apu.dmc.irq);
    }
    apu.write_register(0x4010, 0);
    assert!(!apu.dmc.irq);
}

#[test]
fn length_halt_and_simultaneous_reload_follow_old_counter_value() {
    for &(initial, expected) in &[(0, 254), (1, 0), (2, 1)] {
        let mut apu = Apu::new();
        apu.pulse1.length.enable(true);
        apu.pulse1.length.value = initial;
        apu.frame_cycle = 14912;
        apu.write_register(0x4000, 0x20);
        apu.write_register(0x4003, 0x08);
        apu.step();
        assert_eq!(apu.pulse1.length.value, expected);
        assert!(apu.pulse1.length.halt);
        apu.frame_cycle = 14912;
        apu.step();
        assert_eq!(
            apu.pulse1.length.value, expected,
            "halt applies to later clocks"
        );
    }
}

#[test]
fn envelope_periods_restart_decay_loop_and_constant_volume() {
    for period in 0..16 {
        let mut envelope = Envelope::default();
        envelope.write(period);
        envelope.start = true;
        envelope.clock();
        for level in (0..16).rev() {
            assert_eq!(envelope.output(), level);
            for _ in 0..period {
                envelope.clock();
                assert_eq!(envelope.output(), level);
            }
            envelope.clock();
        }
        assert_eq!(envelope.output(), 0);
        envelope.write(0x20 | period);
        for _ in 0..=period {
            envelope.clock();
        }
        assert_eq!(envelope.output(), 15);
        envelope.write(0x10 | period);
        assert_eq!(envelope.output(), period);
        assert!(
            !envelope.start,
            "control writes do not restart the envelope"
        );
    }
}

#[test]
fn pulse_duty_sequences_and_divider_period() {
    let patterns = [
        [0, 9, 0, 0, 0, 0, 0, 0],
        [0, 9, 9, 0, 0, 0, 0, 0],
        [0, 9, 9, 9, 9, 0, 0, 0],
        [9, 0, 0, 9, 9, 9, 9, 9],
    ];
    for duty in 0..4 {
        let mut pulse = Pulse::default();
        pulse.length.enable(true);
        pulse.write(0, (duty << 6) as u8 | 0x19);
        pulse.write(2, 8);
        pulse.write(3, 0x08);
        pulse.length.commit(false);
        pulse.timer = 8;
        for &expected in &patterns[duty] {
            assert_eq!(pulse.output(), expected);
            for _ in 0..8 {
                pulse.tick();
                assert_eq!(pulse.output(), expected);
            }
            pulse.tick();
        }
        assert_eq!(pulse.position, 0);
        pulse.timer = 5;
        pulse.position = 6;
        pulse.write(3, 0);
        assert_eq!((pulse.position, pulse.timer), (0, 5));
    }
}

#[test]
fn pulse_sweep_negation_mute_and_reload_divider() {
    let mut first = Pulse::default();
    first.first = true;
    let mut second = Pulse::default();
    for pulse in [&mut first, &mut second] {
        pulse.period = 100;
        pulse.write(1, 0x89); // Enabled, subtract, shift 1.
        pulse.clock_sweep();
    }
    assert_eq!((first.period, second.period), (49, 50));
    for pulse in [&mut first, &mut second] {
        pulse.period = 0x600;
        pulse.write(1, 1); // Disabled sweeps still mute overflowing targets.
        assert!(pulse.muted());
        pulse.write(1, 8);
        assert!(!pulse.muted());
        pulse.period = 7;
        assert!(pulse.muted());
        pulse.period = 100;
        pulse.sweep_divider = 2;
        pulse.write(1, 0xb1); // Reload divider, don't apply sweep yet.
        pulse.clock_sweep();
        assert_eq!((pulse.period, pulse.sweep_divider), (100, 3));
        for _ in 0..3 {
            pulse.clock_sweep();
        }
        assert_eq!(pulse.period, 100);
        pulse.clock_sweep();
        assert_eq!(pulse.period, 150);
    }
}

#[test]
fn triangle_linear_counter_waveform_and_held_dac() {
    let mut triangle = Triangle::default();
    triangle.position = 16;
    triangle.length.enable(true);
    triangle.write(0, 0x82);
    triangle.write(2, 1);
    triangle.write(3, 0x08);
    triangle.length.commit(false);
    triangle.clock_linear();
    triangle.clock_linear();
    assert_eq!(triangle.linear, 2, "control bit keeps reloading");
    triangle.write(0, 2);
    triangle.clock_linear();
    triangle.clock_linear();
    assert_eq!(triangle.linear, 1);
    let mut values = vec![];
    for _ in 0..32 {
        values.push(triangle.output());
        triangle.tick();
        triangle.tick();
    }
    assert_eq!(values, (0..16).chain((0..16).rev()).collect::<Vec<u8>>());
    triangle.position = 4;
    triangle.clock_linear();
    for _ in 0..100 {
        triangle.tick();
    }
    assert_eq!(triangle.output(), 11);
    triangle.linear = 2;
    triangle.length.value = 0;
    for _ in 0..100 {
        triangle.tick();
    }
    assert_eq!(triangle.output(), 11);
}

#[test]
fn noise_sequences_have_hardware_periods_and_polarity() {
    for &(short, period) in &[(false, 32767), (true, 93)] {
        let mut noise = Noise::new();
        noise.short = short;
        noise.length.enable(true);
        noise.write(0, 0x1f);
        noise.write(3, 0x08);
        noise.length.commit(false);
        assert_eq!(noise.output(), 0); // LFSR bit 0 suppresses the DAC.
        for step in 1..=period {
            for _ in 0..4 {
                noise.tick();
            }
            if step < period {
                assert_ne!(noise.shift, 1);
            }
        }
        assert_eq!(noise.shift, 1);
        noise.shift = 2;
        assert_eq!(noise.output(), 15);
        noise.length.value = 0;
        assert_eq!(noise.output(), 0);
    }
}

#[test]
fn dmc_decodes_lsb_first_and_preserves_buffer_when_disabled() {
    let mut dmc = Dmc::new();
    dmc.rate = 15;
    dmc.output = 64;
    dmc.remaining = 1;
    dmc.complete_dma(0xa5);
    dmc.enable(false, 0);
    assert_eq!(dmc.buffer, Some(0xa5));
    dmc.bits = 1;
    dmc.timer = 0;
    dmc.tick(); // Finish the silent byte and transfer the sample buffer.
    for expected in [66, 64, 66, 64, 62, 64, 62, 64] {
        for _ in 0..54 {
            dmc.tick();
        }
        assert_eq!(dmc.output, expected);
    }
    assert!(dmc.silent);
    for _ in 0..54 * 16 {
        dmc.tick();
    }
    assert_eq!(dmc.output, 64);
}

#[test]
fn dmc_saturates_without_wrapping_and_uses_all_ntsc_rates() {
    for rate in 0..16 {
        for &(initial, bit, expected) in &[
            (0, 0, 0),
            (1, 0, 1),
            (126, 1, 126),
            (127, 1, 127),
            (2, 0, 0),
            (125, 1, 127),
        ] {
            let mut dmc = Dmc::new();
            dmc.rate = rate;
            dmc.output = initial;
            dmc.shift = bit;
            dmc.silent = false;
            dmc.timer = PERIODS[rate as usize] - 1;
            for _ in 1..PERIODS[rate as usize] {
                dmc.tick();
                assert_eq!(dmc.output, initial);
            }
            dmc.tick();
            assert_eq!(dmc.output, expected);
        }
    }
}

#[test]
fn dmc_address_wrap_length_latching_loops_and_irq() {
    let mut dmc = Dmc::new();
    dmc.write(2, 0xff);
    dmc.write(3, 0xff);
    dmc.enable(true, 0);
    assert_eq!((dmc.address, dmc.remaining), (0xffc0, 4081));
    dmc.write(2, 0);
    dmc.write(3, 0);
    assert_eq!((dmc.address, dmc.remaining), (0xffc0, 4081));
    dmc.address = 0xffff;
    dmc.remaining = 2;
    dmc.write(0, 0x80);
    dmc.complete_dma(0);
    assert_eq!((dmc.address, dmc.remaining, dmc.irq), (0x8000, 1, false));
    dmc.complete_dma(0);
    assert!(dmc.irq);
    dmc.write(0, 0xc0);
    dmc.enable(true, 0);
    dmc.complete_dma(0);
    assert_eq!((dmc.address, dmc.remaining, dmc.irq), (0xc000, 1, false));
}

#[test]
fn mixer_matches_nonlinear_transfer_curves() {
    for p in 0..31 {
        for t in 0..16 {
            for n in 0..16 {
                for d in [0, 1, 63, 64, 126, 127] {
                    let pulse = if p == 0 {
                        0.0
                    } else {
                        95.88 / (8128.0 / p as f64 + 100.0)
                    };
                    let sum = t as f64 / 8227.0 + n as f64 / 12241.0 + d as f64 / 22638.0;
                    let tnd = if sum == 0.0 {
                        0.0
                    } else {
                        159.79 / (1.0 / sum + 100.0)
                    };
                    assert!(
                        (mix(p.min(15), p.saturating_sub(15), t, n, d) as f64 - pulse - tnd).abs()
                            < 0.000001
                    );
                }
            }
        }
    }
    assert!(mix(15, 15, 0, 0, 0) < 2.0 * mix(15, 0, 0, 0, 0));
}

#[test]
fn resampling_is_independent_of_frame_chunk_boundaries() {
    let mut whole = Mixer::new();
    let mut chunked = Mixer::new();
    let mut expected = vec![];
    let mut actual = vec![];
    for cycle in 0..178977 {
        let value = if cycle % 407 < 180 { 0.8 } else { 0.1 };
        whole.tick(value);
        chunked.tick(value);
        if cycle % 17003 == 0 {
            expected.extend(whole.take_samples());
        }
        if cycle % 733 == 0 {
            actual.extend(chunked.take_samples());
        }
    }
    expected.extend(whole.take_samples());
    actual.extend(chunked.take_samples());
    assert_eq!(actual, expected);
    assert!((actual.len() as i32 - 4800).abs() <= 1);
    assert!(actual.iter().any(|&s| s > 1000) && actual.iter().any(|&s| s < -1000));
}

#[test]
fn resampler_rejects_ultrasonic_aliases_and_removes_dc() {
    let mut ultrasonic = Mixer::new();
    let mut dc = Mixer::new();
    for cycle in 0..150000 {
        ultrasonic.tick(if cycle & 1 == 0 { 0.8 } else { 0.0 });
        dc.tick(0.5);
    }
    for samples in [ultrasonic.take_samples(), dc.take_samples()] {
        let tail = &samples[samples.len() - 500..];
        assert!(tail.iter().all(|&s| s.abs() < 20));
    }
}

#[test]
fn cpu_bus_clocks_apu_ppu_and_registers_without_double_clocking() {
    use cpu::Cpu;
    let mut bus = bus(&[0xa9, 1, 0x8d, 0x15, 0x40, 0xa9, 0x18, 0x8d, 3, 0x40]);
    let mut cpu = Cpu::new();
    cpu.reset(&mut bus);
    assert_eq!(bus.cpu_cycles, 7);
    assert_eq!(bus.apu.cycle, 7);
    for expected_cycles in [2, 4, 2, 4] {
        let before = bus.cpu_cycles;
        assert_eq!(cpu.step(&mut bus), expected_cycles);
        assert_eq!(bus.cpu_cycles - before, expected_cycles as u64);
    }
    assert_eq!(bus.cpu_read(0x4015) & 1, 1);
    assert_eq!(bus.apu.pulse1.length.value, 2);
    bus.read_word(0);
    assert_eq!(
        bus.cpu_cycles, 20,
        "inspection reads must not clock peripherals"
    );
    let before = bus.apu.frame_cycle;
    bus.cpu_write(0x4017, 0x80);
    bus.cpu_read(0x4017); // Controller 2 read must not reset the frame counter.
    assert_eq!(bus.apu.frame_cycle, before + 2);
    for _ in 0..5 {
        bus.cpu_read(0);
    }
    assert!(bus.apu.mode5);
}

#[test]
fn dmc_dma_stalls_reads_but_not_writes_and_counts_both_parities() {
    for parity in 0..2 {
        let mut bus = bus(&[0xea]);
        if parity == 1 {
            bus.cpu_write(0, 0);
        }
        bus.apu.dmc.remaining = 1;
        bus.apu.dmc.address = 0xc000;
        bus.apu.dmc.irq_enabled = true;
        let before = bus.cpu_cycles;
        bus.cpu_write(1, 0x55);
        assert_eq!(bus.cpu_cycles - before, 1);
        assert_eq!(bus.apu.dmc.buffer, None);
        let before = bus.cpu_cycles;
        assert_eq!(bus.cpu_read(1), 0x55);
        let stalls = bus.take_stall_cycles();
        assert_eq!(stalls, if before & 1 == 0 { 4 } else { 3 });
        assert_eq!(bus.cpu_cycles - before, stalls as u64 + 1);
        assert_eq!(bus.apu.dmc.buffer, Some(0xea));
        assert_eq!(bus.apu.read_status(0) & 0x90, 0x80);
    }
}

#[test]
fn dmc_steals_oam_gets_without_losing_sprite_bytes() {
    use cpu::Cpu;
    // Sweep requests across CPU writes, OAM halt/alignment, and active gets,
    // including immediate refills when a just-fetched buffer is consumed.
    for parity in 0..2 {
        for delay in 0..12 {
            for timer in [0, 1, 2, 3, 4, 8, 16, 53, 427] {
                let program = if parity == 0 {
                    vec![0xa9, 2, 0x24, 0, 0x8d, 0x14, 0x40]
                } else {
                    vec![0xa9, 2, 0xea, 0x8d, 0x14, 0x40]
                };
                let mut bus = bus(&program);
                for offset in 0..256 {
                    bus.write_word(0x200 + offset, offset as u8);
                }
                let mut cpu = Cpu::new();
                cpu.reset(&mut bus);
                cpu.step(&mut bus);
                cpu.step(&mut bus);
                bus.apu.dmc.remaining = 4;
                bus.apu.dmc.rate = 15;
                bus.apu.dmc.timer = timer;
                bus.apu.dmc.bits = 1;
                bus.apu.dmc.dma_delay = delay;
                let before = bus.cpu_cycles;
                let baseline = 4 + 513 + ((before + 4) & 1) as u16;
                let cycles = cpu.step(&mut bus);
                assert_eq!(bus.cpu_cycles - before, cycles as u64);
                assert!(cycles > baseline, "active DMC must extend sprite DMA");
                assert!(bus.apu.dmc.remaining < 4);
                for offset in 0..256 {
                    bus.write_word(0x2003, offset as u8);
                    assert_eq!(
                        bus.read_word(0x2004),
                        if offset & 3 == 2 {
                            offset as u8 & 0xe3
                        } else {
                            offset as u8
                        }
                    );
                }
            }
        }
    }
}

#[test]
fn irq_asserted_during_sprite_dma_is_serviced_when_cpu_resumes() {
    use cpu::Cpu;
    let mut bus = bus(&[0xa9, 2, 0x58, 0x8d, 0x14, 0x40]);
    let mut cpu = Cpu::new();
    cpu.reset(&mut bus);
    cpu.step(&mut bus); // LDA #2.
    cpu.step(&mut bus); // CLI.
    bus.apu.frame_cycle = 29815;
    cpu.step(&mut bus); // STA $4014 and sprite DMA; IRQ asserts during DMA.
    assert_eq!(cpu.step(&mut bus), 7);
    assert_eq!(cpu.step(&mut bus), 5);
    assert_eq!(bus.read_word(0), 1);
}

#[test]
fn apu_irq_enters_real_cpu_handler_after_cli_delay() {
    use cpu::Cpu;
    let mut bus = bus(&[0x58, 0xea, 0xea]); // CLI, NOP, interrupted NOP.
    let mut cpu = Cpu::new();
    cpu.reset(&mut bus);
    bus.apu.dmc.irq = true;
    assert_eq!(cpu.step(&mut bus), 2); // CLI uses the previous I flag.
    assert_eq!(cpu.step(&mut bus), 2);
    assert_eq!(cpu.step(&mut bus), 7);
    assert_eq!(cpu.step(&mut bus), 5); // INC $00 in IRQ handler.
    assert_eq!(bus.read_word(0), 1);
    assert_eq!(bus.read_word(0x1fd), 0xc0);
    assert_eq!(bus.read_word(0x1fc), 2);
    assert_eq!(bus.read_word(0x1fb) & 0x34, 0x20);
}

#[test]
fn real_cpu_frames_produce_expected_pulse_triangle_and_dmc_pitches() {
    use cpu::Cpu;
    use joypad::ButtonState;
    use nes::Nes;
    let cases: &[(&[(u16, u8)], f64, i16)] = &[
        (
            &[
                (0x4015, 1),
                (0x4000, 0xbf),
                (0x4001, 8),
                (0x4002, 253),
                (0x4003, 8),
            ],
            CPU_HZ / (16.0 * 254.0),
            350,
        ),
        (
            &[(0x4015, 4), (0x4008, 0xff), (0x400a, 253), (0x400b, 8)],
            CPU_HZ / (32.0 * 254.0),
            200,
        ),
        (
            &[
                (0x4010, 0x4f),
                (0x4011, 64),
                (0x4012, 0x10),
                (0x4013, 0),
                (0x4015, 16),
            ],
            CPU_HZ / (8.0 * 54.0),
            50,
        ),
    ];
    for &(registers, expected_hz, threshold) in cases {
        let mut program = vec![0xa9, 0x40, 0x8d, 0x17, 0x40];
        for &(address, value) in registers {
            program.extend_from_slice(&[0xa9, value, 0x8d, address as u8, (address >> 8) as u8]);
        }
        let loop_address = 0xc000 + program.len() as u16;
        program.extend_from_slice(&[0x4c, loop_address as u8, (loop_address >> 8) as u8]);
        program.resize(0x401, 0xea);
        program[0x400] = 0x0f; // Repeated DMC byte: four up steps, four down.
        let mut nes = Nes {
            cpu: Cpu::new(),
            interconnect: bus(&program),
        };
        nes.reset();
        let mut samples = vec![];
        for _ in 0..100 {
            nes.run_frame(ButtonState {
                a: false,
                b: false,
                select: false,
                start: false,
                up: false,
                down: false,
                left: false,
                right: false,
            });
            samples.extend(nes.interconnect.apu.take_samples());
        }
        let expected_count = nes.interconnect.cpu_cycles as f64 * SAMPLE_RATE as f64 / CPU_HZ;
        assert!((samples.len() as f64 - expected_count).abs() < 1.0);
        let stable = &samples[4800..]; // Skip startup and the filter transient.
        let mut armed = false;
        let mut crossings = 0;
        for &sample in stable {
            if sample < -threshold {
                armed = true;
            }
            if armed && sample > threshold {
                crossings += 1;
                armed = false;
            }
        }
        let measured_hz = crossings as f64 * SAMPLE_RATE as f64 / stable.len() as f64;
        assert!(
            (measured_hz - expected_hz).abs() < 1.5,
            "expected {:.3} Hz, got {:.3} Hz",
            expected_hz,
            measured_hz
        );
    }
}
