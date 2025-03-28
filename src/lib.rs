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
    // callbacks: HashMap<String, Arc<Mutex<Box<dyn Callback<dyn std::any::Any + Send + 'static> + Send + 'static>>>>,
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

/*
pub trait Callback {
    fn call(&self, params: Vec<CallbackParam>) -> Result<CallbackResponse, CallbackError>;
}

struct F1<T: Fn() -> ()>(T);

impl<T> Callback for F1<T>
    where T : Fn() -> ()
{
    fn call(&self, _params: Vec<CallbackParam>) -> Result<CallbackResponse, CallbackError>
    {
        (self.0)();
        Ok(CallbackResponse::Void)
    }
}

struct F2<T: Fn(i32) -> ()>(T);

impl<T> Callback for F2<T>
    where T : Fn(i32) -> ()
{
    fn call(&self, params: Vec<CallbackParam>) -> Result<CallbackResponse, CallbackError>
    {
        if params.len() != 1 {
            return Err(CallbackError::Message("foobar".to_string()));
        }

        let fst = &params[0];
        let fst = if let CallbackParam::Int(fst) = fst { fst } else {
            return Err(CallbackError::Message("foobar2".to_string()));
        };

        (self.0)(*fst);
        Ok(CallbackResponse::Void)
    }
}

*/
/*
struct F2<T: fn_ops::Fn<i32>>(T);

impl<T> Callback for F2<T>
    where T : fn_ops::Fn<i32>
{
    fn call(&self, _params: Vec<CallbackParam>) -> Result<CallbackResponse, CallbackError>
    {
        self.0.call(32);
        Ok(CallbackResponse::Void)
    }
}
*/

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

impl Into<ParamType> for &CallbackParam {
    fn into(self) -> ParamType {
        match self {
            CallbackParam::Int(_) => ParamType::Int,
            CallbackParam::Boolean(_) => ParamType::Boolean,
            _ => todo!(),
        }
    }
}

impl SandkasseImports for State {
    fn registered_callback(&mut self, name: String, params: Vec<CallbackParam>) -> Result<CallbackResponse, CallbackError> {
        let callback = self.callbacks.get(&name).expect("get");
        let callback = callback.lock().unwrap();

        let res = (callback.inner)(Params { inner: VecDeque::from(params) });
        Ok(res.expect("res"))

        /*
        let param_types = params.iter().map(|i| i.into()).collect::<Vec<ParamType>>();

        if params.len() == 0 {
            let callback = callback.downcast_ref::<Box<dyn Callback<()> + Send + 'static>>().expect("downcast");
            return Ok(callback.call(Params { inner: VecDeque::from(params), }).expect("foobar"));
        } else if param_types == vec![ParamType::Int] {
            let callback = callback.downcast_ref::<Box<dyn Callback<i32> + Send + 'static>>().expect("downcast");
            return Ok(callback.call(Params { inner: VecDeque::from(params), }).expect("foobar"));
        } else if param_types == vec![ParamType::Boolean] {
            let callback = callback.downcast_ref::<Box<dyn Callback<bool> + Send + 'static>>().expect("downcast");
            return Ok(callback.call(Params { inner: VecDeque::from(params), }).expect("foobar"));
        } else if param_types == vec![ParamType::Int, ParamType::Int] {
            let callback = callback.downcast_ref::<Box<dyn Callback<(i32, i32)> + Send + 'static>>().expect("downcast");
            return Ok(callback.call(Params { inner: VecDeque::from(params), }).expect("foobar"));
        }
        */

        // todo!("register...");
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

    pub fn register<P: std::any::Any + ToParamList + Send + 'static>(&mut self, name: String, callback: impl Callback<P> + Send + 'static) -> Result<(), Error>
    {
        /*
        let callback: Box<dyn Callback<P> + Send + 'static> = Box::new(callback);
        let callback: Box<dyn std::any::Any + Send + 'static> = Box::new(callback);
        */

        let callback = callback.to_callable();
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

pub trait Callback<P: ?Sized + std::any::Any + 'static> : Send
{
    fn to_callable(self) -> Callable;
}

pub struct Callable {
    inner: Box<dyn Fn(Params) -> Result<CallbackResponse, Error> + Send>,
}

/*
    fn call(&self, mut params: Params) -> Result<CallbackResponse, Error> {
        let x: Box<dyn Fn(Params) -> CallbackResponse> = Box::new(|mut p| {
            let res = (self as &dyn Fn(P) -> R)(P::from(&mut p));
            res.into_response() });
*/

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

impl<R, F, P1, P2> Callback<(P1, P2)> for F
where
    F: Fn(P1, P2) -> R + Send + 'static,
    P1: FromParams + 'static,
    P2: FromParams + 'static,
    R: IntoCallbackResponse,
{
    fn to_callable(self) -> Callable {
        let inner: Box<dyn Fn(Params) -> Result<CallbackResponse, Error> + Send> = Box::new(move |mut p| {
            let res = (&self as&dyn Fn(P1, P2) -> R)(P1::from(&mut p), P2::from(&mut p));
            Ok(res.into_response()) });

        Callable { inner }
    }
    /*
    fn call(&self, mut params: Params) -> Result<CallbackResponse, Error> {
        Ok((self as &dyn Fn(P1, P2) -> R)(P1::from(&mut params), P2::from(&mut params)).into_response())
    }
    */
}

impl<R, P, F> Callback<P> for F
where
    F: Fn(P) -> R + Send + 'static,
    P: FromParams + 'static,
    R: IntoCallbackResponse,
{
    /*
    fn call(&self, mut params: Params) -> Result<CallbackResponse, Error> {
        Ok((self as &dyn Fn(P) -> R)(P::from(&mut params)).into_response())
    }
    */
    fn to_callable(self) -> Callable {
        let inner: Box<dyn Fn(Params) -> Result<CallbackResponse, Error> + Send> = Box::new(move |mut p| {
            let res = (&self as&dyn Fn(P) -> R)(P::from(&mut p));
            Ok(res.into_response()) });

        Callable { inner }
    }
}

impl<R, F> Callback<()> for F
where
    F: Fn() -> R + Send + 'static,
    (): std::any::Any + Send + 'static,
    R: IntoCallbackResponse,
{
    fn to_callable(self) -> Callable {
        let inner: Box<dyn Fn(Params) -> Result<CallbackResponse, Error> + Send> = Box::new(move |_p| {
            let res = (&self as&dyn Fn() -> R)();
            Ok(res.into_response()) });

        Callable { inner }
    }
    /*
    fn call(&self, _params: Params) -> Result<CallbackResponse, Error> {
        Ok((self as &dyn Fn() -> R)().into_response())
    }
    */
}

