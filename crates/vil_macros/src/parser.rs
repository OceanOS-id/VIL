use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Ident, LitStr, Token,
};

/// Represents the entire `vil_workflow!` block.
pub struct WorkflowDef {
    pub name: LitStr,
    pub hosts: Vec<HostDef>,
    pub processes: Vec<Ident>,
    pub instances: Vec<InstanceDef>,
    pub routes: Vec<RouteDef>,
    pub failovers: Vec<FailoverDef>,
}

pub struct HostDef {
    pub name: Ident,
    pub address: LitStr,
}

pub struct InstanceDef {
    pub name: Ident,
    pub host: Option<Ident>,
}

pub struct RouteDef {
    pub src_process: Ident,
    pub src_port: Ident,
    pub dst_process: Ident,
    pub dst_port: Ident,
    pub transfer_mode: Ident, // LoanWrite, LoanRead, Copy, Move
    pub transport: Option<Ident>,
}

pub struct FailoverDef {
    pub source: Ident,
    pub target: Ident, // process name OR 'retry'
    pub retry_attempts: Option<syn::LitInt>,
    pub retry_backoff: Option<syn::LitInt>,
    pub condition: Ident,
    pub strategy: Option<Ident>, // usually Immediate
}

// parsing implementation
mod kw {
    syn::custom_keyword!(name);
    syn::custom_keyword!(hosts);
    syn::custom_keyword!(processes);
    syn::custom_keyword!(instances);
    syn::custom_keyword!(routes);
    syn::custom_keyword!(failover);
}

impl Parse for WorkflowDef {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut name_opt = None;
        let mut hosts_opt = None;
        let mut processes_opt = None;
        let mut instances_opt = None;
        let mut routes_opt = None;
        let mut failovers_opt = None;

        while !input.is_empty() {
            let lookahead = input.lookahead1();
            if lookahead.peek(kw::name) {
                input.parse::<kw::name>()?;
                input.parse::<Token![:]>()?;
                name_opt = Some(input.parse::<LitStr>()?);
                if input.peek(Token![,]) {
                    input.parse::<Token![,]>()?;
                }
            } else if lookahead.peek(kw::hosts) {
                input.parse::<kw::hosts>()?;
                input.parse::<Token![:]>()?;
                let content;
                syn::bracketed!(content in input);
                let parsed_hosts: Punctuated<HostDef, Token![,]> =
                    content.parse_terminated(HostDef::parse, Token![,])?;
                hosts_opt = Some(parsed_hosts.into_iter().collect());
                if input.peek(Token![,]) {
                    input.parse::<Token![,]>()?;
                }
            } else if lookahead.peek(kw::processes) {
                input.parse::<kw::processes>()?;
                input.parse::<Token![:]>()?;
                let content;
                syn::bracketed!(content in input);
                let parsed_processes: Punctuated<Ident, Token![,]> =
                    content.parse_terminated(Ident::parse, Token![,])?;
                processes_opt = Some(parsed_processes.into_iter().collect());
                if input.peek(Token![,]) {
                    input.parse::<Token![,]>()?;
                }
            } else if lookahead.peek(kw::instances) {
                input.parse::<kw::instances>()?;
                input.parse::<Token![:]>()?;
                let content;
                syn::bracketed!(content in input);
                let parsed_instances: Punctuated<InstanceDef, Token![,]> =
                    content.parse_terminated(InstanceDef::parse, Token![,])?;
                instances_opt = Some(parsed_instances.into_iter().collect());
                if input.peek(Token![,]) {
                    input.parse::<Token![,]>()?;
                }
            } else if lookahead.peek(kw::routes) {
                input.parse::<kw::routes>()?;
                input.parse::<Token![:]>()?;
                let content;
                syn::bracketed!(content in input);
                let parsed_routes: Punctuated<RouteDef, Token![,]> =
                    content.parse_terminated(RouteDef::parse, Token![,])?;
                routes_opt = Some(parsed_routes.into_iter().collect());
                if input.peek(Token![,]) {
                    input.parse::<Token![,]>()?;
                }
            } else if lookahead.peek(kw::failover) {
                input.parse::<kw::failover>()?;
                input.parse::<Token![:]>()?;
                let content;
                syn::bracketed!(content in input);
                let parsed_failovers: Punctuated<FailoverDef, Token![,]> =
                    content.parse_terminated(FailoverDef::parse, Token![,])?;
                failovers_opt = Some(parsed_failovers.into_iter().collect());
                if input.peek(Token![,]) {
                    input.parse::<Token![,]>()?;
                }
            } else {
                return Err(lookahead.error());
            }
        }

        let name = name_opt.ok_or_else(|| input.error("Missing `name` field in workflow"))?;
        let hosts = hosts_opt.unwrap_or_default();
        let processes = processes_opt.unwrap_or_default();
        let instances = instances_opt.unwrap_or_default();
        let routes = routes_opt.unwrap_or_default();
        let failovers = failovers_opt.unwrap_or_default();

        Ok(Self {
            name,
            hosts,
            processes,
            instances,
            routes,
            failovers,
        })
    }
}

impl Parse for HostDef {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;
        input.parse::<Token![:]>()?;
        let host_kw: Ident = input.parse()?;
        if host_kw != "Host" {
            return Err(syn::Error::new(host_kw.span(), "Expected `Host`"));
        }
        let content;
        syn::parenthesized!(content in input);
        let address: LitStr = content.parse()?;
        Ok(Self { name, address })
    }
}

impl Parse for InstanceDef {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;
        let mut host = None;
        if input.peek(Token![@]) {
            input.parse::<Token![@]>()?;
            host = Some(input.parse()?);
        }
        Ok(Self { name, host })
    }
}

impl Parse for RouteDef {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let src_process: Ident = input.parse()?;
        input.parse::<Token![.]>()?;
        let src_port: Ident = input.parse()?;
        input.parse::<Token![->]>()?;
        let dst_process: Ident = input.parse()?;
        input.parse::<Token![.]>()?;
        let dst_port: Ident = input.parse()?;

        let content;
        syn::parenthesized!(content in input);
        let transfer_mode: Ident = content.parse()?;

        let mut transport = None;
        if content.peek(Token![,]) {
            content.parse::<Token![,]>()?;
            let transport_kw: Ident = content.parse()?;
            if transport_kw != "transport" {
                return Err(syn::Error::new(transport_kw.span(), "Expected `transport`"));
            }
            content.parse::<Token![:]>()?;
            transport = Some(content.parse()?);
        }

        Ok(Self {
            src_process,
            src_port,
            dst_process,
            dst_port,
            transfer_mode,
            transport,
        })
    }
}

// target => retry(3, backoff: 100ms) (on: TransferFailed, strategy: Immediate)
// primary => backup (on: HostDown, strategy: Immediate)
impl Parse for FailoverDef {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let source: Ident = input.parse()?;
        input.parse::<Token![=>]>()?;

        let target: Ident = input.parse()?;

        let mut retry_attempts = None;
        let mut retry_backoff = None;

        if target == "retry" {
            let retry_content;
            syn::parenthesized!(retry_content in input);
            retry_attempts = Some(retry_content.parse()?);
            if retry_content.peek(Token![,]) {
                retry_content.parse::<Token![,]>()?;
                let backoff_kw: Ident = retry_content.parse()?;
                if backoff_kw != "backoff" {
                    return Err(syn::Error::new(backoff_kw.span(), "Expected `backoff`"));
                }
                retry_content.parse::<Token![:]>()?;
                retry_backoff = Some(retry_content.parse()?);
                // Allow `100ms` syntax. We simplify by only supporting a LitInt
                // millisecond value (e.g. `100`). If "ms" appears as an Ident
                // after the value, just consume it.
                if retry_content.peek(Ident) {
                    let unit: Ident = retry_content.parse()?;
                    if unit != "ms" {
                        return Err(syn::Error::new(unit.span(), "Expected `ms`"));
                    }
                }
            }
        }

        let content;
        syn::parenthesized!(content in input);

        let on_kw: Ident = content.parse()?;
        if on_kw != "on" {
            return Err(syn::Error::new(on_kw.span(), "Expected `on`"));
        }
        content.parse::<Token![:]>()?;
        let condition: Ident = content.parse()?;

        let mut strategy = None;
        if content.peek(Token![,]) {
            content.parse::<Token![,]>()?;
            let strategy_kw: Ident = content.parse()?;
            if strategy_kw != "strategy" {
                return Err(syn::Error::new(strategy_kw.span(), "Expected `strategy`"));
            }
            content.parse::<Token![:]>()?;
            strategy = Some(content.parse()?);
        }

        Ok(Self {
            source,
            target,
            retry_attempts,
            retry_backoff,
            condition,
            strategy,
        })
    }
}
