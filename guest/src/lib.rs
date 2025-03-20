use std::collections::HashMap;

use rquickjs::{Context, Function, Result as RQuickJsResult, Runtime};

use exports::havarnov::sandkasse::runtime::{
    Error, EvalParams, Guest, GuestCtx, RegisterParams, Response, ResponseType,
};

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

fn handle_registered<'a>(
    s: String,
    ctx: &rquickjs::Ctx<'a>,
    input: impl rquickjs::IntoJs<'a>,
) -> i32 {
    let value = input.into_js(ctx);
    println!("called {} with {:?}", s, value);
    666
}

impl std::convert::From<rquickjs::Error> for exports::havarnov::sandkasse::runtime::Error {
    fn from(e: rquickjs::Error) -> Self {
        exports::havarnov::sandkasse::runtime::Error::Message(format!("{:?}", e))
    }
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
                    Function::new(ctx.clone(), move |input: String| {
                        handle_registered(name.clone(), &ctx, input)
                    })?
                    .with_name(params.name.to_string())?
                } else {
                    Function::new(ctx.clone(), move |input: i32| {
                        handle_registered(name.clone(), &ctx, input)
                    })?
                    .with_name(params.name.to_string())?
                },
            )?;
            Ok(())
        })?;
        Ok(true)
    }

    fn eval(&self, req: EvalParams) -> Result<Response, Error> {
        let value = match req.response_type {
            ResponseType::Void => self.ctx.with(|ctx| -> RQuickJsResult<Response> {
                _ = ctx.eval::<(), _>(req.source)?;
                Ok(Response::Void)
            })?,
            ResponseType::Int => self.ctx.with(|ctx| -> RQuickJsResult<Response> {
                let value = ctx.eval::<i32, _>(req.source)?;
                Ok(Response::Int(value))
            })?,
            ResponseType::Boolean => self.ctx.with(|ctx| -> RQuickJsResult<Response> {
                let value = ctx.eval::<bool, _>(req.source)?;
                Ok(Response::Boolean(value))
            })?,
            ResponseType::Str => self.ctx.with(|ctx| -> RQuickJsResult<Response> {
                let value = ctx.eval::<String, _>(req.source)?;
                Ok(Response::Str(value))
            })?,
        };
        Ok(value)
    }
}
