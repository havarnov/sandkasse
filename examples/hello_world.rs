use sandkasse::Runtime;

fn p() {
    println!("p");
}

fn main() {
    let mut runtime = Runtime::new().expect("runtime");
    let mut ctx = runtime.create_ctx().expect("ctx");

    ctx.eval::<()>(format!("function yalla(v) {{ return v * 2; }}"))
        .expect("eval");

    let v: i32 = ctx.eval(format!("yalla(45);")).expect("eval");
    println!("value: {:?}", v);

    let v: bool = ctx
        .eval(format!("let f = () => {{ return true; }}; f();"))
        .expect("eval");
    println!("value: {:?}", v);

    let v: String = ctx.eval(format!("\"string from js\";")).expect("eval");
    println!("value: {:?}", v);

    ctx.register("x".to_string(), || { println!("hello from the other side"); }).expect("register");
    ctx.eval::<()>(format!("x();")).expect("eval");

    ctx.register("y".to_string(), |i: i32| { println!("hello from the other side: {:?}", i); }).expect("register");
    ctx.eval::<()>(format!("y(42);")).expect("eval");
}
