use sandkasse::Runtime;
fn print() {
    println!("print");
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

    ctx.register("z".to_string(), |i: i32, j: i32| { println!("hello from the other side: {:?}, {:?}", i, j); }).expect("register");
    ctx.eval::<()>(format!("z(42, 22);")).expect("eval");

    ctx.register("add".to_string(), |i: i32, j: i32| { i + j }).expect("register");
    let v = ctx.eval::<i32>(format!("add(42, 22);")).expect("eval");
    println!("value: {:?}", v);

    ctx.register("print".to_string(), print).expect("register");
    ctx.eval::<()>(format!("print();")).expect("eval");

    ctx.register("a".to_string(), |i: bool| { println!("bool from the other side: {:?}", i); }).expect("register");
    ctx.eval::<()>(format!("a(false);")).expect("eval");
}
