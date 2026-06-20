use std::collections::VecDeque;

pub struct IRQCommand {
    pub irq_line: u32,
    pub value: bool,
}

impl IRQCommand {
    pub fn new(irq_line: u32, value: bool) -> Self {
        Self { irq_line, value }
    }
}

impl Clone for IRQCommand {
    fn clone(&self) -> Self {
        Self {
            irq_line: self.irq_line,
            value: self.value,
        }
    }
}

pub struct IRQHandler {
    commands: VecDeque<IRQCommand>,
}

impl IRQHandler {
    pub fn new() -> Self {
        Self {
            commands: VecDeque::new(),
        }
    }

    pub fn trigger_irq(&mut self, irq: IRQCommand) {
        self.commands.push_back(irq);
    }

    pub fn handle_irqs(&mut self) -> VecDeque<IRQCommand> {
        std::mem::take(&mut self.commands)
    }
}
