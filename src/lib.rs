use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use wasmtime::component::*;
use wasmtime::component::{Linker, ResourceTable};
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi::{IoView, WasiCtx, WasiCtxBuilder, WasiView};

use exports::havarnov::sandkasse::runtime::*;

bindgen!({
    path: ".",
    world: "sandkasse",
    additional_derives: [PartialEq],
});

struct State {
    ctx: WasiCtx,
    table: ResourceTable,
    callbacks: HashMap<String, Arc<Mutex<Callable>>>,
}

impl IoView for State {
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
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

pub struct Context<'a> {
    store: &'a mut Store<State>,
    ctx: GuestCtx<'a>,
    resource: ResourceAny,
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

        let mut linker = Linker::<State>::new(&engine);
        wasmtime_wasi::add_to_linker_sync(&mut linker)?;
        Sandkasse::add_to_linker(&mut linker, |state: &mut State| state)?;

        let mut builder = WasiCtxBuilder::new();

        // TODO: config
        builder.inherit_stdio();

        let mut store = Store::new(
            &engine,
            State {
                ctx: builder.build(),
                table: ResourceTable::new(),
                callbacks: HashMap::new(),
            },
        );
        // store.set_fuel(4_000_000).expect("set fuel");

       let package = Sandkasse::instantiate(&mut store, &component, &linker)?;

        Ok(Runtime { store, package, })
    }

    pub fn create_ctx<'a>(&'a mut self) -> Result<Context<'a>, Error> {
        let ctx = self.package.interface0.ctx();
        let resource = ctx.call_constructor(&mut self.store)?;

        Ok(Context {
            store: &mut self.store,
            ctx: ctx,
            resource: resource,
        })
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
    fn response_type() -> ResponseType;
}

impl FromJs for () {
    fn from_js(value: Value) -> Result<Self, Error> {
        match value {
            Value::Void => Ok(()),
            _ => Err(Error::WrongType("expected void".to_string())),
        }
    }

    fn response_type() -> ResponseType {
        ResponseType::Void
    }
}

impl FromJs for bool {
    fn from_js(value: Value) -> Result<Self, Error> {
        match value {
            Value::Bool(v) => Ok(v),
            _ => Err(Error::WrongType("expected bool".to_string())),
        }
    }

    fn response_type() -> ResponseType {
        ResponseType::Boolean
    }
}

impl FromJs for i32 {
    fn from_js(value: Value) -> Result<Self, Error> {
        match value {
            Value::Int(int) => Ok(int),
            _ => Err(Error::WrongType("expected int".to_string())),
        }
    }

    fn response_type() -> ResponseType {
        ResponseType::Int
    }
}

impl FromJs for String {
    fn from_js(value: Value) -> Result<Self, Error> {
        match value {
            Value::Str(v) => Ok(v),
            _ => Err(Error::WrongType("expected string".to_string())),
        }
    }

    fn response_type() -> ResponseType {
        ResponseType::Str
    }
}

impl SandkasseImports for State {
    fn registered_callback(&mut self, name: String, params: Vec<CallbackParam>) -> Result<CallbackResponse, CallbackError> {
        let callback = self.callbacks.get(&name).expect("get");
        let callback = callback.lock().unwrap();

        let res = (callback.inner)(Params { inner: VecDeque::from(params) });
        Ok(res.expect("res"))
    }
}

impl Context<'_> {
    pub fn eval<V: FromJs>(&mut self, source: String) -> Result<V, Error> {
        let request = EvalParams {
            source,
            response_type: V::response_type(),
        };
        let response = self
            .ctx
            .call_eval(&mut self.store, self.resource, &request)?;
        match response {
            Ok(Response::Void) => V::from_js(Value::Void),
            Ok(Response::Int(v)) => V::from_js(Value::Int(v)),
            Ok(Response::Boolean(v)) => V::from_js(Value::Bool(v)),
            Ok(Response::Str(v)) => V::from_js(Value::Str(v)),
            Err(e) => Err(Error::WrongType(format!("{:?}", e))),
        }
    }

    pub fn register<P: std::any::Any + ToParamList + Send + 'static>(&mut self, name: String, callback: impl IntoCallback<P> + Send + 'static) -> Result<(), Error>
    {
        let callback = callback.into_callable();
        self.store.data_mut().callbacks.insert(name.clone(), Arc::new(Mutex::new(callback)));

        let param_types = P::get_param_list();
        let request = RegisterParams { name, param_types };
        let _response = self
            .ctx
            .call_register(&mut self.store, self.resource, &request)?;
        Ok(())
    }
}

pub trait ToParamList {
    fn get_param_list() -> Vec<ParamType>;
}

impl ToParamList for () {
    fn get_param_list() -> Vec<ParamType>
    {
        vec![]
    }
}

impl<P> ToParamList for P
    where P: FromParams
{
    fn get_param_list() -> Vec<ParamType>
    {
        vec![P::get_param_type()]
    }
}

impl<P, U> ToParamList for (P, U)
    where P: FromParams, U: FromParams
{
    fn get_param_list() -> Vec<ParamType>
    {
        vec![P::get_param_type(), U::get_param_type()]
    }
}

pub struct Params
{
    inner: VecDeque<CallbackParam>,
}

pub trait FromParams {
    fn from(params: &mut Params) -> Self where Self: Sized;
    fn get_param_type() -> ParamType where Self: Sized;
}

pub trait IntoCallbackResponse {
    fn into_response(&self) -> CallbackResponse;
}

pub trait IntoCallback<P: ?Sized + std::any::Any + 'static> : Send
{
    fn into_callable(self) -> Callable;
}

pub struct Callable {
    inner: Box<dyn Fn(Params) -> Result<CallbackResponse, Error> + Send>,
}

impl IntoCallbackResponse for () {
    fn into_response(&self) -> CallbackResponse {
        CallbackResponse::Void
    }
}

impl IntoCallbackResponse for i32 {
    fn into_response(&self) -> CallbackResponse {
        CallbackResponse::Int(*self)
    }
}

impl FromParams for String {
    fn from(params: &mut Params) -> Self
    {
        match params.inner.pop_front() {
            Some(CallbackParam::Str(i)) => i,
            i => todo!("{:?}", i),
        }
    }

    fn get_param_type() -> ParamType {
        ParamType::Str
    }
}

impl FromParams for bool {
    fn from(params: &mut Params) -> Self
    {
        match params.inner.pop_front() {
            Some(CallbackParam::Boolean(i)) => i,
            i => todo!("{:?}", i),
        }
    }

    fn get_param_type() -> ParamType {
        ParamType::Boolean
    }
}

impl FromParams for i32 {
    fn from(params: &mut Params) -> Self
    {
        match params.inner.pop_front() {
            Some(CallbackParam::Int(i)) => i,
            i => todo!("{:?}", i),
        }
    }

    fn get_param_type() -> ParamType {
        ParamType::Int
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
        let inner: Box<dyn Fn(Params) -> Result<CallbackResponse, Error> + Send> = Box::new(move |mut p| {
            let res = (&self as&dyn Fn(P1, P2) -> R)(P1::from(&mut p), P2::from(&mut p));
            Ok(res.into_response()) });

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
        let inner: Box<dyn Fn(Params) -> Result<CallbackResponse, Error> + Send> = Box::new(move |mut p| {
            let res = (&self as&dyn Fn(P) -> R)(P::from(&mut p));
            Ok(res.into_response()) });

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
        let inner: Box<dyn Fn(Params) -> Result<CallbackResponse, Error> + Send> = Box::new(move |_p| {
            let res = (&self as&dyn Fn() -> R)();
            Ok(res.into_response()) });

        Callable { inner }
    }
}

