use rquickjs::{Context, Function, Result as RQuickJsResult, Runtime, IntoJs};

use exports::havarnov::sandkasse::runtime::{
    Error, EvalParams, Guest, GuestCtx, RegisterParams, Response, ResponseType, ParamType
};

wit_bindgen::generate!({ // W: call to unsafe function `_export_call_cabi` is unsafe and requires unsafe block: call to unsafe function
    path: "..",
    world: "sandkasse",
    additional_derives: [PartialEq],
});

struct Component;

export!(Component);

impl Guest for Component {
    type Ctx = RuntimeCtx;
}

struct RuntimeCtx {
    ctx: Context,
}

impl<'a> IntoJs<'a> for CallbackResponse {
    fn into_js(self, ctx: &rquickjs::Ctx<'a>) -> Result<rquickjs::Value<'a>, rquickjs::Error> {
        match self {
            CallbackResponse::Void => Ok(rquickjs::Value::new_undefined(ctx.clone())),
            CallbackResponse::Int(value) => Ok(rquickjs::Value::new_int(ctx.clone(), value)),
            CallbackResponse::Str(value) => rquickjs::String::from_str(ctx.clone(), &value).map(|s| { s.into_value() }),
            _ => todo!("rquickjs::into_js"),
        }
    }
}

fn handle_registered2<'a>(
    s: String,
    ctx: &rquickjs::Ctx<'a>,
    input1: impl rquickjs::IntoJs<'a>,
    input2: impl rquickjs::IntoJs<'a>,
) -> impl rquickjs::IntoJs<'a>
{
    let value = input1.into_js(ctx).expect("into_js");
    let mut params = if value.is_undefined() {
        vec![]
    }
    else if value.is_int() {
        vec![CallbackParam::Int(value.as_int().unwrap())]
    }
    else if value.is_string() {
        vec![CallbackParam::Str(value.as_string().unwrap().to_string().unwrap())]
    }
    else {
        todo!("value to params");
    };

    let value = input2.into_js(ctx).expect("into_js");
    if value.is_undefined() {
        ()
    }
    else if value.is_int() {
        params.push(CallbackParam::Int(value.as_int().unwrap()));
    }
    else if value.is_string() {
        params.push(CallbackParam::Str(value.as_string().unwrap().to_string().unwrap()));
    }
    else {
        todo!("value to params");
    };

    let response = registered_callback(&s, &params).expect("registered_callback");
    response
}

fn handle_registered<'a>(
    s: String,
    ctx: &rquickjs::Ctx<'a>,
    input: impl rquickjs::IntoJs<'a>,
) -> impl rquickjs::IntoJs<'a>
{
    let value = input.into_js(ctx).expect("into_js");
    let params = if value.is_undefined() {
        vec![]
    }
    else if value.is_int() {
        vec![CallbackParam::Int(value.as_int().unwrap())]
    }
    else if value.is_bool() {
        vec![CallbackParam::Boolean(value.as_bool().unwrap())]
    }
    else if value.is_string() {
        vec![CallbackParam::Str(value.as_string().unwrap().to_string().unwrap())]
    }
    else {
        todo!("value to params");
    };
    let response = registered_callback(&s, &params).expect("registered_callback");
    response
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
                if params.param_types.len() == 0 {
                    Function::new(ctx.clone(), move || {
                        handle_registered(name.clone(), &ctx, ())
                    })?
                    .with_name(&params.name)?
                } else if params.param_types == vec![ParamType::Int] {
                    Function::new(ctx.clone(), move |i: i32| {
                        handle_registered(name.clone(), &ctx, i)
                    })?
                    .with_name(&params.name)?
                } else if params.param_types == vec![ParamType::Boolean] {
                    Function::new(ctx.clone(), move |i: bool| {
                        handle_registered(name.clone(), &ctx, i)
                    })?
                    .with_name(&params.name)?
                } else if params.param_types == vec![ParamType::Str] {
                    Function::new(ctx.clone(), move |i: String| {
                        handle_registered(name.clone(), &ctx, i)
                    })?
                    .with_name(&params.name)?
                } else if params.param_types == vec![ParamType::Int, ParamType::Int] {
                    Function::new(ctx.clone(), move |i: i32, j: i32| {
                        handle_registered2(name.clone(), &ctx, i, j)
                    })?
                    .with_name(&params.name)?
                } else
                {
                    todo!("uouo")
                }
            )?;
            Ok(())
        })?;
        Ok(true)
    }

    fn eval(&self, req: EvalParams) -> Result<Response, Error> {
        let value = match req.response_type {
            ResponseType::Void => self.ctx.with(|ctx| -> RQuickJsResult<Response> {
                ctx.eval::<(), _>(req.source)?;
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
