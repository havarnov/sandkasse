use std::sync::{Mutex, OnceLock};

use protocol::*;

use rquickjs::{Context, Function, Result as RQuickJsResult, Runtime};

wit_bindgen::generate!({ // W: call to unsafe function `_export_call_cabi` is unsafe and requires unsafe block: call to unsafe function
    path: "..",
    world: "sandkasse",
    additional_derives: [PartialEq],
});

struct RuntimeCtx {
    ctx: Context,
}

struct RuntimeWrapper;

export!(RuntimeWrapper);

impl Guest for RuntimeWrapper {
    fn eval(payload: Vec<u8>) -> Result<Vec<u8>, Vec<u8>> {
        let request: EvalRequest = rmp_serde::from_slice(&payload).expect("from_slice");
        let ctx = context().lock().unwrap().ctx.clone();
        let response = match request.response_type {
            EvalResponseType::Void => ctx.with(|ctx| -> RQuickJsResult<EvalResponse> {
                ctx.eval::<(), _>(request.source)?;
                Ok(EvalResponse::Void)
            }),
            EvalResponseType::Int => ctx.with(|ctx| -> RQuickJsResult<EvalResponse> {
                let value = ctx.eval::<i32, _>(request.source)?;
                Ok(EvalResponse::Int(value))
            }),
            EvalResponseType::Bool => ctx.with(|ctx| -> RQuickJsResult<EvalResponse> {
                let value = ctx.eval::<bool, _>(request.source)?;
                Ok(EvalResponse::Bool(value))
            }),
            EvalResponseType::String => ctx.with(|ctx| -> RQuickJsResult<EvalResponse> {
                let value = ctx.eval::<String, _>(request.source)?;
                Ok(EvalResponse::String(value))
            }),
        };

        if let Ok(response) = response {
            let bytes = rmp_serde::to_vec(&response).expect("to_vec()");
            Ok(bytes)
        } else {
            todo!("panic?")
        }
    }

    fn register(payload: Vec<u8>) -> Result<Vec<u8>, Vec<u8>> {
        let request: RegisterRequest = rmp_serde::from_slice(&payload).expect("from_slice");
        context()
            .lock()
            .unwrap()
            .ctx
            .with(|ctx| -> RQuickJsResult<()> {
                let global = ctx.globals();
                global.set(
                    request.name.to_string(),
                    Function::new(
                        ctx.clone(),
                        CallbackHandler {
                            name: request.name.to_string(),
                        },
                    )?
                    .with_name(&request.name)?,
                )?;
                Ok(())
            })
            .expect("with");
        Ok(vec![])
    }
}

unsafe impl Send for RuntimeCtx {}

fn context() -> &'static Mutex<RuntimeCtx> {
    static RUNTIME_CTX: OnceLock<Mutex<RuntimeCtx>> = OnceLock::new();
    RUNTIME_CTX.get_or_init(|| {
        let rt = Runtime::new().expect("create runtime");
        let context = Context::full(&rt).expect("create context");
        Mutex::new(RuntimeCtx { ctx: context })
    })
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
                callback_params.push(CallRegisteredParam::Int(value.as_int().unwrap()));
            } else if value.is_string() {
                callback_params.push(CallRegisteredParam::String(
                    value.as_string().unwrap().to_string().unwrap(),
                ));
            } else if value.is_bool() {
                callback_params.push(CallRegisteredParam::Bool(value.as_bool().unwrap()));
            } else {
                todo!("value to params");
            };
        }

        let req = CallRegisteredRequest {
            name: self.name.to_string(),
            params: callback_params,
        };

        let bytes = rmp_serde::to_vec(&req).expect("to_vec()");

        let response = registered_callback(&bytes).expect("registered_callback");

        let eval_response: EvalResponse = rmp_serde::from_slice(&response).expect("from_slice");

        let ctx = params.ctx();
        match eval_response {
            EvalResponse::Void => Ok(rquickjs::Value::new_undefined(ctx.clone())),
            EvalResponse::Int(value) => Ok(rquickjs::Value::new_int(ctx.clone(), value)),
            EvalResponse::Bool(value) => Ok(rquickjs::Value::new_bool(ctx.clone(), value)),
            EvalResponse::String(value) => {
                rquickjs::String::from_str(ctx.clone(), &value).map(|s| s.into_value())
            }
        }
    }
}
