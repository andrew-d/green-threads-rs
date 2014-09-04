#![feature(phase)]

#[phase(plugin, link)] extern crate green_threads;

#[inline(never)]
fn info(x: &int, y: &int) {
    println!("In foobar({}, {})", x, y);
}

green!(fn foobar(x: int, y: int) -> int {
    info(&x, &y);
    x + y - 1
})

fn main() {
    println!("This is the example");
    println!("Result is: {}", foobar(10, 11));
}
