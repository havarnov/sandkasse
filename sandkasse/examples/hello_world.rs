use sandkasse::Runtime;

fn main() {
    let mut runtime = Runtime::new().expect("runtime");
    let mut ctx = runtime.create_ctx().expect("ctx");
    ctx.eval(format!("1 + 1;")).expect("eval");
    ctx.eval(format!("1 + 32;")).expect("eval");

    // POC
    // ctx.register("print", Value::String, |value| { println!("{}", value); });
    ctx.register("yolo".to_string(), true);
    ctx.register("yalla".to_string(), false);

    ctx.eval(format!("yolo(\"YOLO_INPUT\"); 21")).expect("eval");
    ctx.eval(format!("yalla(45); 21;")).expect("eval");

    ctx.eval(format!("yalla(45);")).expect("eval");
}
