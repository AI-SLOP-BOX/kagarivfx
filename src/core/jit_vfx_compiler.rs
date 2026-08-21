#![allow(dead_code)]
/// Bytecode OpCodes for JIT compiled VFX Expressions & Effect Stacks.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JitOpCode {
    PushConst(f32),
    LoadTime,
    LoadVar(u32),
    Add,
    Sub,
    Mul,
    Sin,
    Cos,
}

/// JIT Compiled Bytecode Execution Pipeline.
#[derive(Debug, Clone)]
pub struct JitCompiledProgram {
    pub instructions: Vec<JitOpCode>,
}

impl JitCompiledProgram {
    /// Executes JIT compiled instructions using a high-speed stack machine.
    pub fn execute(&self, time_sec: f32, vars: &[f32]) -> f32 {
        let mut stack = Vec::with_capacity(16);

        for op in &self.instructions {
            match op {
                JitOpCode::PushConst(val) => stack.push(*val),
                JitOpCode::LoadTime => stack.push(time_sec),
                JitOpCode::LoadVar(idx) => stack.push(vars.get(*idx as usize).copied().unwrap_or(0.0)),
                JitOpCode::Add => {
                    let b = stack.pop().unwrap_or(0.0);
                    let a = stack.pop().unwrap_or(0.0);
                    stack.push(a + b);
                }
                JitOpCode::Sub => {
                    let b = stack.pop().unwrap_or(0.0);
                    let a = stack.pop().unwrap_or(0.0);
                    stack.push(a - b);
                }
                JitOpCode::Mul => {
                    let b = stack.pop().unwrap_or(0.0);
                    let a = stack.pop().unwrap_or(0.0);
                    stack.push(a * b);
                }
                JitOpCode::Sin => {
                    let a = stack.pop().unwrap_or(0.0);
                    stack.push(a.sin());
                }
                JitOpCode::Cos => {
                    let a = stack.pop().unwrap_or(0.0);
                    stack.push(a.cos());
                }
            }
        }

        stack.pop().unwrap_or(0.0)
    }
}

/// JIT Compiler Engine: Compiles procedural expressions into optimized bytecode instruction streams.
pub struct JitVfxCompiler;

impl JitVfxCompiler {
    /// Compiles a sine-wave wiggle program: `sin(time * freq) * amp`
    pub fn compile_sine_wave(freq: f32, amp: f32) -> JitCompiledProgram {
        JitCompiledProgram {
            instructions: vec![
                JitOpCode::LoadTime,
                JitOpCode::PushConst(freq),
                JitOpCode::Mul,
                JitOpCode::Sin,
                JitOpCode::PushConst(amp),
                JitOpCode::Mul,
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jit_sine_wave_execution() {
        let prog = JitVfxCompiler::compile_sine_wave(2.0 * std::f32::consts::PI, 100.0);
        let val0 = prog.execute(0.0, &[]);
        let val_quarter = prog.execute(0.25, &[]);

        assert!((val0 - 0.0).abs() < 0.001);
        assert!((val_quarter - 100.0).abs() < 0.1);
    }
}
