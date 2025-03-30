use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use wasmtime::component::*;
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi::{IoView, WasiCtx, WasiCtxBuilder, WasiView};

use protocol::*;

bindgen!({
    path: ".",
    world: "sandkasse",
    additional_derives: [PartialEq],
});

struct State {
    ctx: WasiCtx,
    resource_table: ResourceTable,
    callbacks: HashMap<String, Arc<Mutex<Callable>>>,
}

impl IoView for State {
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.resource_table
    }
}
impl WasiView for State {
    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.ctx
    }
}

#[derive(Debug)]
pub enum Error {
    Init(String),
    WrongType(String),
}

impl From<wasmtime::Error> for Error {
    fn from(error: wasmtime::Error) -> Self {
        Self::Init(format!("{:?}", error))
    }
}

pub struct Runtime {
    store: Store<State>,
    package: Sandkasse,
}

impl Runtime {
    pub fn new() -> Result<Self, Error> {
        let mut config = Config::default();
        config.wasm_component_model(true);

        // TODO: configuration
        // config.debug_info(true);
        // config.consume_fuel(true);

        let engine = Engine::new(&config)?;

        // TODO: load from binary
        let bytes = include_bytes!("../guest/target/wasm32-wasip1/release/guest.wasm");
        let component = Component::from_binary(&engine, bytes)?;

        let mut linker: Linker<State> = Linker::new(&engine);
        wasmtime_wasi::add_to_linker_sync(&mut linker)?;
        Sandkasse::add_to_linker(&mut linker, |state: &mut State| state)?;

        let mut builder = WasiCtxBuilder::new();

        // TODO: config
        builder.inherit_stdio();

        let mut store = Store::new(
            &engine,
            State {
                ctx: builder.build(),
                resource_table: ResourceTable::new(),
                callbacks: HashMap::new(),
            },
        );
        // store.set_fuel(4_000_000).expect("set fuel");
        //
        let package = Sandkasse::instantiate(&mut store, &component, &linker)?;

        Ok(Runtime { store, package })
    }
}

pub enum Value {
    Void,
    Int(i32),
    Bool(bool),
    Str(String),
}

pub trait FromJs: Sized {
    fn from_js(value: Value) -> Result<Self, Error>;
    fn response_type() -> EvalResponseType;
}

impl FromJs for () {
    fn from_js(value: Value) -> Result<Self, Error> {
        match value {
            Value::Void => Ok(()),
            _ => Err(Error::WrongType("expected void".to_string())),
        }
    }

    fn response_type() -> EvalResponseType {
        EvalResponseType::Void
    }
}

impl FromJs for bool {
    fn from_js(value: Value) -> Result<Self, Error> {
        match value {
            Value::Bool(v) => Ok(v),
            _ => Err(Error::WrongType("expected bool".to_string())),
        }
    }

    fn response_type() -> EvalResponseType {
        EvalResponseType::Bool
    }
}

impl FromJs for i32 {
    fn from_js(value: Value) -> Result<Self, Error> {
        match value {
            Value::Int(int) => Ok(int),
            _ => Err(Error::WrongType("expected int".to_string())),
        }
    }

    fn response_type() -> EvalResponseType {
        EvalResponseType::Int
    }
}

impl FromJs for String {
    fn from_js(value: Value) -> Result<Self, Error> {
        match value {
            Value::Str(v) => Ok(v),
            _ => Err(Error::WrongType("expected string".to_string())),
        }
    }

    fn response_type() -> EvalResponseType {
        EvalResponseType::String
    }
}

impl SandkasseImports for State {
    fn registered_callback(&mut self, payload: Vec<u8>) -> Result<Vec<u8>, Vec<u8>> {
        let request: CallRegisteredRequest = rmp_serde::from_slice(&payload).expect("from_slice");

        let callback = self.callbacks.get(&request.name).expect("get");
        let callback = callback.lock().unwrap();

        let res = (callback.inner)(Params {
            inner: VecDeque::from(request.params),
        });
        let res = res.expect("res");
        let res = rmp_serde::to_vec(&res).expect("to_vec");
        Ok(res)
    }
}

impl Runtime {
    pub fn eval<V: FromJs>(&mut self, source: String) -> Result<V, Error> {
        let request = EvalRequest {
            source,
            response_type: V::response_type(),
        };

        let bytes = rmp_serde::to_vec(&request).expect("to_vec()");
        let response = self
            .package
            .call_eval(&mut self.store, &bytes)
            .expect("call_eval")
            .expect("call_eval_inner");
        let eval_response: Result<EvalResponse, _> = rmp_serde::from_slice(&response);

        match eval_response {
            Ok(EvalResponse::Void) => V::from_js(Value::Void),
            Ok(EvalResponse::Int(v)) => V::from_js(Value::Int(v)),
            Ok(EvalResponse::Bool(v)) => V::from_js(Value::Bool(v)),
            Ok(EvalResponse::String(v)) => V::from_js(Value::Str(v)),
            Err(e) => Err(Error::WrongType(format!("{:?}", e))),
        }
    }

    pub fn register<P: 'static>(
        &mut self,
        name: &str,
        callback: impl IntoCallback<P> + Send + 'static,
    ) -> Result<(), Error> {
        let callback = callback.into_callable();
        self.store
            .data_mut()
            .callbacks
            .insert(name.to_string(), Arc::new(Mutex::new(callback)));

        let request = RegisterRequest {
            name: name.to_string(),
        };
        let bytes = rmp_serde::to_vec(&request).expect("to_vec()");
        let response = self
            .package
            .call_register(&mut self.store, &bytes)
            .expect("call_eval")
            .expect("call_eval_inner");
        assert!(response.len() == 0);

        Ok(())
    }
}

pub struct Params {
    inner: VecDeque<CallRegisteredParam>,
}

pub trait FromParams {
    fn from(params: &mut Params) -> Self
    where
        Self: Sized;
}

pub trait IntoCallbackResponse {
    fn into_response(&self) -> EvalResponse;
}

pub trait IntoCallback<P: ?Sized + std::any::Any + 'static>: Send {
    fn into_callable(self) -> Callable;
}

pub struct Callable {
    inner: Box<dyn Fn(Params) -> Result<EvalResponse, Error> + Send>,
}

impl IntoCallbackResponse for () {
    fn into_response(&self) -> EvalResponse {
        EvalResponse::Void
    }
}

impl IntoCallbackResponse for i32 {
    fn into_response(&self) -> EvalResponse {
        EvalResponse::Int(*self)
    }
}

impl FromParams for String {
    fn from(params: &mut Params) -> Self {
        match params.inner.pop_front() {
            Some(CallRegisteredParam::String(i)) => i,
            i => todo!("{:?}", i),
        }
    }
}

impl FromParams for bool {
    fn from(params: &mut Params) -> Self {
        match params.inner.pop_front() {
            Some(CallRegisteredParam::Bool(i)) => i,
            i => todo!("{:?}", i),
        }
    }
}

impl FromParams for i32 {
    fn from(params: &mut Params) -> Self {
        match params.inner.pop_front() {
            Some(CallRegisteredParam::Int(i)) => i,
            i => todo!("{:?}", i),
        }
    }
}

impl<R, F, P1, P2> IntoCallback<(P1, P2)> for F
where
    F: Fn(P1, P2) -> R + Send + 'static,
    P1: FromParams + 'static,
    P2: FromParams + 'static,
    R: IntoCallbackResponse,
{
    fn into_callable(self) -> Callable {
        let inner: Box<dyn Fn(Params) -> Result<EvalResponse, Error> + Send> =
            Box::new(move |mut p| {
                let res = (&self as &dyn Fn(P1, P2) -> R)(P1::from(&mut p), P2::from(&mut p));
                Ok(res.into_response())
            });

        Callable { inner }
    }
}

impl<R, P, F> IntoCallback<P> for F
where
    F: Fn(P) -> R + Send + 'static,
    P: FromParams + 'static,
    R: IntoCallbackResponse,
{
    fn into_callable(self) -> Callable {
        let inner: Box<dyn Fn(Params) -> Result<EvalResponse, Error> + Send> =
            Box::new(move |mut p| {
                let res = (&self as &dyn Fn(P) -> R)(P::from(&mut p));
                Ok(res.into_response())
            });

        Callable { inner }
    }
}

impl<R, F> IntoCallback<()> for F
where
    F: Fn() -> R + Send + 'static,
    (): std::any::Any + Send + 'static,
    R: IntoCallbackResponse,
{
    fn into_callable(self) -> Callable {
        let inner: Box<dyn Fn(Params) -> Result<EvalResponse, Error> + Send> =
            Box::new(move |_p| {
                let res = (&self as &dyn Fn() -> R)();
                Ok(res.into_response())
            });

        Callable { inner }
    }
}
