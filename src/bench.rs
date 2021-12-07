use web_sys::console;

/// adopted from https://rustwasm.github.io/book/game-of-life/time-profiling.html#time-each-universetick-with-consoletime-and-consoletimeend
pub struct Timer<'a> {
    name: &'a str,
}

impl<'a> Timer<'a> {
    pub fn new(name: &'a str) -> Timer<'a> {
        console::time_with_label(name);
        Timer { name }
    }
}

impl<'a> Drop for Timer<'a> {
    fn drop(&mut self) {
        console::time_end_with_label(self.name);
    }
}
