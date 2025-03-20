A rust library to run javascript in a completely safe sandbox.

```rust
let mut runtime = Runtime::new().expect("runtime");
let mut ctx = runtime.create_ctx().expect("ctx");

ctx.eval::<()>(format!("function yalla(v) {{ return v * 2; }}")).expect("eval");

let v: i32 = ctx.eval(format!("yalla(45);")).expect("eval");
assert!(v == 90);

let v: bool = ctx.eval(format!("let f = () => {{ return true; }}; f();")).expect("eval");
assert!(v == true);

let v: String = ctx.eval(format!("\"string from js\";")).expect("eval");
assert!(v == "string from js".to_string());
```
