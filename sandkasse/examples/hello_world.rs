use sandkasse::Runtime;

fn main() {
    let mut runtime = Runtime::new().expect("runtime");
    let mut ctx = runtime.create_ctx().expect("ctx");
    ctx.eval::<i32>(format!("1 + 1;")).expect("eval");
    ctx.eval::<i32>(format!("1 + 32;")).expect("eval");

    // POC
    // ctx.register("print", |s: String| { println!("{}", s); });
    ctx.register("yolo".to_string(), true);
    ctx.register("yalla".to_string(), false);

    ctx.eval::<i32>(format!("yolo(\"YOLO_INPUT\"); 21")).expect("eval");
    ctx.eval::<i32>(format!("yalla(45); 21;")).expect("eval");

    ctx.eval::<i32>(format!("yalla(45);")).expect("eval");
    let v: i32 = ctx.eval(format!("yalla(45);")).expect("eval");
    println!("value: {:?}", v);

    // ctx.register("double", |i: i32| { i * 2 });
    // let value = ctx.eval::<i32>(format!("double(42);"))?;
    // assert!(value == 84);
}
