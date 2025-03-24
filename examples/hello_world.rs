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

    // POC
    ctx.register("double".to_string(), || { println!("hello from the other side"); }).expect("register");
    // ctx.register("double".to_string(), p);
    ctx.eval::<()>(format!("double();")).expect("eval");
    // let value = ctx.eval::<i32>(format!("double(42);"))?;
    // assert!(value == 84);
    ctx.register_2("triple".to_string(), move |i: i32| { println!("hello from the other side: {:?}", i); }).expect("register");
    ctx.eval::<()>(format!("triple(32);")).expect("eval");
}
