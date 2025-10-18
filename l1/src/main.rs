mod l1;
mod parser;

fn main() {
    let _x = l1::Program::new(String::from("foo"), Vec::new());
    let _y = l1::Function::new(String::from("bar"), 0, 0, Vec::new());
}
