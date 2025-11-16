pub mod cpu;
pub mod opcode;

fn main() {
    let mut cpu = cpu::Cpu::new();
    loop {
        cpu.step();
        cpu.tick_timers();
        // renderer.draw(&cpu.display);
        // input.update(&mut cpu.keys);
    }
}
