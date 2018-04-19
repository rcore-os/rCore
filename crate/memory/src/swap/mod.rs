pub mod fifo;
mod mock_page_table;

type Addr = usize;

trait SwapManager<T: PageTable> {
    /// Create and initialize for the swap manager
    fn new(page_table: &'static T) -> Self;
    /// Called when tick interrupt occured
    fn tick(&mut self);
    /// Called when map a swappable page into the memory
    fn push(&mut self, addr: Addr);
    /// Called to delete the addr entry from the swap manager
    fn pop(&mut self, addr: Addr);
    /// Try to swap out a page, return then victim
    fn swap(&mut self) -> Option<Addr>;
}

trait PageTable {
    fn accessed(&self, addr: Addr) -> bool;
    fn dirty(&self, addr: Addr) -> bool;
}