use ferrumvm::irq::{handler::{IRQCommand, IRQHandler}, map::IrqMap};

#[test]
fn new_handler_has_no_pending_irqs() {
    let mut handler = IRQHandler::new();
    assert!(handler.handle_irqs().is_empty());
}

#[test]
fn trigger_queues_irq_in_order() {
    let mut handler = IRQHandler::new();
    handler.trigger_irq(IRQCommand::new(4, true));
    handler.trigger_irq(IRQCommand::new(4, false));
    handler.trigger_irq(IRQCommand::new(5, true));

    let irqs = handler.handle_irqs();
    let collected: Vec<(u32, bool)> = irqs.into_iter().map(|c| (c.irq_line, c.value)).collect();
    assert_eq!(collected, vec![(4, true), (4, false), (5, true)]);
}

#[test]
fn handle_irqs_drains_queue() {
    let mut handler = IRQHandler::new();
    handler.trigger_irq(IRQCommand::new(1, true));
    let _ = handler.handle_irqs();
    assert!(handler.handle_irqs().is_empty());
}

#[test]
fn irq_command_is_cloneable() {
    let cmd = IRQCommand::new(3, true);
    let clone = cmd.clone();
    assert_eq!(clone.irq_line, 3);
    assert!(clone.value);
}

#[test]
fn stores_fields_and_reads_them_back() {
    let map = IrqMap::new(4, 2, 0);
    assert_eq!(map.read_gsi(), 4);
    assert_eq!(map.read_irq_pin(), 2);
    assert_eq!(map.read_irq_chip(), 0);
}

#[test]
fn default_map_has_expected_structure() {
    let map = IrqMap::default_map();
    assert_eq!(map.len(), 7);
    assert_eq!(map[0].read_gsi(), 0);
    assert_eq!(map[4].read_irq_pin(), 5); // virtio-blk
    // every entry uses the I/O APIC (chip 0) with pin equal to gsi
    for entry in map {
        assert_eq!(entry.read_irq_chip(), 0);
        assert_eq!(entry.read_irq_pin(), entry.read_gsi());
    }
}
