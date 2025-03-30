use sandkasse::Runtime;
fn print() {
    println!("print");
}

fn main() {
    let mut runtime = Runtime::new().expect("runtime");

    runtime
        .eval::<()>(format!("function yalla(v) {{ return v * 2; }}"))
        .expect("eval");

    let v: i32 = runtime.eval(format!("yalla(45);")).expect("eval");
    println!("value: {:?}", v);

    let v: bool = runtime
        .eval(format!("let f = () => {{ return true; }}; f();"))
        .expect("eval");
    println!("value: {:?}", v);

    let v: String = runtime.eval(format!("\"string from js\";")).expect("eval");
    println!("value: {:?}", v);

    runtime
        .register("x", || {
            println!("hello from the other side");
        })
        .expect("register");
    runtime.eval::<()>(format!("x();")).expect("eval");

    runtime
        .register("y", |i: i32| {
            println!("hello from the other side: {:?}", i);
        })
        .expect("register");
    runtime.eval::<()>(format!("y(42);")).expect("eval");

    runtime
        .register("z", |i: i32, j: i32| {
            println!("hello from the other side: {:?}, {:?}", i, j);
        })
        .expect("register");
    runtime.eval::<()>(format!("z(42, 22);")).expect("eval");

    runtime
        .register("add", |i: i32, j: i32| i + j)
        .expect("register");
    let v = runtime.eval::<i32>(format!("add(42, 22);")).expect("eval");
    println!("value: {:?}", v);

    runtime.register("print", print).expect("register");
    runtime.eval::<()>(format!("print();")).expect("eval");

    runtime
        .register("a", |i: bool| {
            println!("bool from the other side: {:?}", i);
        })
        .expect("register");
    runtime.eval::<()>(format!("a(false);")).expect("eval");

    runtime
        .register("s", |i: String| {
            println!("str from the other side: {:?}", i);
        })
        .expect("register");
    runtime.eval::<()>(format!("s(\"yalla\");")).expect("eval");
}
