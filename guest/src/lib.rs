use std::collections::HashMap;

use rquickjs::{Context, Runtime, Function, Result as RQuickJsResult};

use exports::havarnov::sandkasse::runtime::{Guest, GuestCtx, Request, Response, Error};

wit_bindgen::generate!({ // W: call to unsafe function `_export_call_cabi` is unsafe and requires unsafe block: call to unsafe function
    path: "..",
    world: "sandkasse",
});

struct Component {
    ctx: Context,
}

export!(Component);

impl Guest for Component {
    type Ctx = RuntimeCtx;
}

struct RuntimeCtx {
    ctx: Context,
}

fn handle_registered<'a>(s: String, ctx: &rquickjs::Ctx<'a>, input: impl rquickjs::IntoJs<'a>) -> i32 {
    let value = input.into_js(ctx);
    println!("called {} with {:?}", s, value);
    666
}

impl std::convert::From<rquickjs::Error> for exports::havarnov::sandkasse::runtime::Error {
    fn from(_: rquickjs::Error) -> Self { todo!() }
}

impl GuestCtx for RuntimeCtx {
    fn new() -> Self {
        let rt = Runtime::new().expect("create runtime");
        let context = Context::full(&rt).expect("create context");
        RuntimeCtx { ctx: context }
    }

    fn handle(&self, req: Request) -> Result<Response, Error> {
        match req {
            Request::Eval(input) => {
                let v = self.ctx.with(|ctx| {
                    ctx.eval::<i32, _>(input).expect("woot")
                });
                Ok(Response::Todo(format!("{:?}", v)))
            },
            Request::Register((name, is_string)) => {
                let x = name.to_string();
                self.ctx.with(|ctx| -> RQuickJsResult<()> {
                    let global = ctx.globals();
                    global.set(
                        name.to_string(),
                        if is_string {
                            Function::new(ctx.clone(), move |input: String| handle_registered(x.clone(), &ctx, input))?.with_name(name.to_string())?
                        } else {
                            Function::new(ctx.clone(), move |input: i32| handle_registered(x.clone(), &ctx, input))?.with_name(name.to_string())?
                        },
                    )?;
                    Ok(())
                })?;
                Ok(Response::Todo(format!("Registered a new function: {}", name)))
            },
        }
    }
}
