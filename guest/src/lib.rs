use std::collections::HashMap;

use rquickjs::{Context, Runtime};

use exports::plugnplay::plugnplay::runtime::{Guest, GuestCtx, Request, Response, Error};

wit_bindgen::generate!({ // W: call to unsafe function `_export_call_cabi` is unsafe and requires unsafe block: call to unsafe function
    path: "..",
    world: "plugnplay",
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

impl GuestCtx for RuntimeCtx {
    fn new() -> Self {
        let rt = Runtime::new().expect("create runtime");
        let context = Context::full(&rt).expect("create context");
        RuntimeCtx { ctx: context }
    }

    fn eval(&self, req: Request) -> Result<Response, Error> {
        match req {
            Request::B(input) => {
                let v = self.ctx.with(|ctx| {
                    ctx.eval::<i32, _>(input).expect("woot")
                });
                Ok(Response::B(format!("result: {:?}", v)))
            },
            Request::A => Ok(Response::A {}),
        }
    }
}
