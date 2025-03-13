use safejs::Runtime;

fn main() {
    let mut runtime = Runtime::new().expect("runtime");
    let mut ctx = runtime.create_ctx().expect("ctx");
    ctx.eval(format!("1 + 1;")).expect("eval");
    ctx.eval(format!("1 + 32;")).expect("eval");
}
