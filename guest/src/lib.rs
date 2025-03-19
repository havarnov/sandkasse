use std::collections::HashMap;

use rquickjs::{Context, Runtime, Function, Result as RQuickJsResult};

use exports::havarnov::sandkasse::runtime::{Guest, GuestCtx, Request, Response, Error, RegisterParams};

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

    fn register(&self, params: RegisterParams) -> Result<bool, Error> {
        let name = params.name.clone();
        self.ctx.with(|ctx| -> RQuickJsResult<()> {
            let global = ctx.globals();
            global.set(
                params.name.to_string(),
                if params.is_int {
                    Function::new(ctx.clone(), move |input: String| handle_registered(name.clone(), &ctx, input))?.with_name(params.name.to_string())?
                } else {
                    Function::new(ctx.clone(), move |input: i32| handle_registered(name.clone(), &ctx, input))?.with_name(params.name.to_string())?
                },
            )?;
            Ok(())
        })?;
        Ok(true)
    }

    fn handle(&self, req: Request) -> Result<Response, Error> {
        match req {
            Request::Eval(input) => {
                let v = self.ctx.with(|ctx| {
                    ctx.eval::<i32, _>(input).expect("woot")
                });
                Ok(Response::Int(v))
            },
        }
    }
}
