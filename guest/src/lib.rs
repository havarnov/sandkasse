use rquickjs::{Context, Function, IntoJs, Result as RQuickJsResult, Runtime};

use exports::havarnov::sandkasse::runtime::{
    Error, EvalParams, Guest, GuestCtx, RegisterParams, Response, ResponseType,
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
            CallbackResponse::Str(value) => {
                rquickjs::String::from_str(ctx.clone(), &value).map(|s| s.into_value())
            }
            _ => todo!("rquickjs::into_js"),
        }
    }
}

struct CallbackHandler {
    name: String,
}

impl<'js> rquickjs::function::IntoJsFunc<'js, CallbackHandler> for CallbackHandler {
    fn param_requirements() -> rquickjs::function::ParamRequirement {
        rquickjs::function::ParamRequirement::any()
    }

    fn call<'a>(
        &self,
        params: rquickjs::function::Params<'a, 'js>,
    ) -> Result<rquickjs::Value<'js>, rquickjs::Error> {
        let mut callback_params = vec![];

        for i in 0..params.len() {
            let value = params.arg(i).expect("arg");

            if value.is_int() {
                callback_params.push(CallbackParam::Int(value.as_int().unwrap()));
            } else if value.is_string() {
                callback_params.push(CallbackParam::Str(
                    value.as_string().unwrap().to_string().unwrap(),
                ));
            } else if value.is_bool() {
                callback_params.push(CallbackParam::Boolean(value.as_bool().unwrap()));
            } else {
                todo!("value to params");
            };
        }

        let response =
            registered_callback(&self.name, &callback_params).expect("registered_callback");
        response.into_js(params.ctx())
    }
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
        self.ctx.with(|ctx| -> RQuickJsResult<()> {
            let global = ctx.globals();
            global.set(
                params.name.to_string(),
                Function::new(
                    ctx.clone(),
                    CallbackHandler {
                        name: params.name.to_string(),
                    },
                )?
                .with_name(&params.name)?,
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
