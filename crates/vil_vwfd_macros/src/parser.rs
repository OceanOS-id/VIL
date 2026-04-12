//! Parser — parse vil_vwfd! macro syntax into intermediate representation.

use syn::parse::{Parse, ParseStream};
use syn::{Ident, LitStr, Token, Result};

/// Top-level macro definition.
pub struct VwfdMacroDef {
    pub id: String,
    pub name: Option<String>,
    pub trigger: TriggerDef,
    pub activities: Vec<ActivityDef>,
    pub flow: Vec<String>,             // ordered node IDs: trigger -> a -> b -> end
    pub durability: Option<String>,    // "eventual", "immediate", "non_durable"
}

pub struct TriggerDef {
    pub trigger_type: String,          // "webhook" | "cron"
    pub route: String,                 // "/orders"
    pub method: Option<String>,        // "POST"
    pub response_mode: Option<String>, // "buffered" | "streaming"
    pub end_activity: Option<String>,  // "respond"
    pub cron_expr: Option<String>,     // "5s"
}

pub struct ActivityDef {
    pub id: String,
    pub kind: ActivityKind,
    pub connector_ref: Option<String>,
    pub operation: Option<String>,
    pub mappings: Vec<MappingDef>,
    pub output: Option<String>,
    pub durability: Option<String>,
    pub compensation: Option<CompensationDef>,
    pub response_expr: Option<(String, String)>, // (language, source) for EndTrigger
    pub trigger_ref: Option<String>,
}

pub enum ActivityKind {
    Connector,
    EndTrigger,
    Transform,
    VilRules(String), // rule_set_id
}

pub struct MappingDef {
    pub target: String,
    pub language: String,
    pub source: String,
}

pub struct CompensationDef {
    pub connector_ref: String,
    pub operation: String,
    pub mappings: Vec<MappingDef>,
}

// ── Custom keywords ──
mod kw {
    syn::custom_keyword!(id);
    syn::custom_keyword!(name);
    syn::custom_keyword!(trigger);
    syn::custom_keyword!(activities);
    syn::custom_keyword!(flow);
    syn::custom_keyword!(durability);
    syn::custom_keyword!(webhook);
    syn::custom_keyword!(cron);
    syn::custom_keyword!(connector);
    syn::custom_keyword!(end_trigger);
    syn::custom_keyword!(transform);
    syn::custom_keyword!(rules);
    syn::custom_keyword!(operation);
    syn::custom_keyword!(input);
    syn::custom_keyword!(output);
    syn::custom_keyword!(method);
    syn::custom_keyword!(response_mode);
    syn::custom_keyword!(end_activity);
    syn::custom_keyword!(response);
    syn::custom_keyword!(compensation);
    syn::custom_keyword!(literal);
    syn::custom_keyword!(cel);
    syn::custom_keyword!(spv1);
    syn::custom_keyword!(vil_query);
}

impl Parse for VwfdMacroDef {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut id = None;
        let mut name = None;
        let mut trigger = None;
        let mut activities = Vec::new();
        let mut flow = Vec::new();
        let mut durability = None;

        while !input.is_empty() {
            let lookahead = input.lookahead1();

            if lookahead.peek(kw::id) {
                input.parse::<kw::id>()?;
                input.parse::<Token![:]>()?;
                id = Some(input.parse::<LitStr>()?.value());
            } else if lookahead.peek(kw::name) {
                input.parse::<kw::name>()?;
                input.parse::<Token![:]>()?;
                name = Some(input.parse::<LitStr>()?.value());
            } else if lookahead.peek(kw::durability) {
                input.parse::<kw::durability>()?;
                input.parse::<Token![:]>()?;
                durability = Some(input.parse::<Ident>()?.to_string());
            } else if lookahead.peek(kw::trigger) {
                input.parse::<kw::trigger>()?;
                input.parse::<Token![:]>()?;
                trigger = Some(parse_trigger(input)?);
            } else if lookahead.peek(kw::activities) {
                input.parse::<kw::activities>()?;
                input.parse::<Token![:]>()?;
                let content;
                syn::braced!(content in input);
                activities = parse_activities(&content)?;
            } else if lookahead.peek(kw::flow) {
                input.parse::<kw::flow>()?;
                input.parse::<Token![:]>()?;
                flow = parse_flow(input)?;
            } else {
                return Err(input.error("expected: id, name, trigger, activities, flow, or durability"));
            }

            // Optional trailing comma
            if input.peek(Token![,]) { input.parse::<Token![,]>()?; }
        }

        Ok(VwfdMacroDef {
            id: id.ok_or_else(|| input.error("missing 'id'"))?,
            name,
            trigger: trigger.ok_or_else(|| input.error("missing 'trigger'"))?,
            activities,
            flow,
            durability,
        })
    }
}

fn parse_trigger(input: ParseStream) -> Result<TriggerDef> {
    let lookahead = input.lookahead1();
    if lookahead.peek(kw::webhook) {
        input.parse::<kw::webhook>()?;
        let route_content;
        syn::parenthesized!(route_content in input);
        let route = route_content.parse::<LitStr>()?.value();

        let mut method = None;
        let mut response_mode = None;
        let mut end_activity = None;

        if input.peek(syn::token::Brace) {
            let content;
            syn::braced!(content in input);
            while !content.is_empty() {
                let key = content.parse::<Ident>()?;
                content.parse::<Token![:]>()?;
                match key.to_string().as_str() {
                    "method" => { method = Some(content.parse::<Ident>()?.to_string()); }
                    "response_mode" => { response_mode = Some(content.parse::<Ident>()?.to_string()); }
                    "end_activity" => { end_activity = Some(content.parse::<LitStr>()?.value()); }
                    _ => { let _: Ident = content.parse()?; }
                }
                if content.peek(Token![,]) { content.parse::<Token![,]>()?; }
            }
        }

        Ok(TriggerDef {
            trigger_type: "webhook".into(), route, method,
            response_mode, end_activity, cron_expr: None,
        })
    } else if lookahead.peek(kw::cron) {
        input.parse::<kw::cron>()?;
        let content;
        syn::parenthesized!(content in input);
        let expr = content.parse::<LitStr>()?.value();
        Ok(TriggerDef {
            trigger_type: "cron".into(), route: String::new(),
            method: None, response_mode: None, end_activity: None,
            cron_expr: Some(expr),
        })
    } else {
        Err(input.error("expected webhook(...) or cron(...)"))
    }
}

fn parse_activities(input: ParseStream) -> Result<Vec<ActivityDef>> {
    let mut activities = Vec::new();
    while !input.is_empty() {
        let id = input.parse::<Ident>()?.to_string();
        input.parse::<Token![=>]>()?;
        let activity = parse_activity(id, input)?;
        activities.push(activity);
        if input.peek(Token![,]) { input.parse::<Token![,]>()?; }
    }
    Ok(activities)
}

fn parse_activity(id: String, input: ParseStream) -> Result<ActivityDef> {
    let lookahead = input.lookahead1();

    if lookahead.peek(kw::connector) {
        input.parse::<kw::connector>()?;
        let ref_content;
        syn::parenthesized!(ref_content in input);
        let connector_ref = ref_content.parse::<LitStr>()?.value();

        let mut operation = None;
        let mut mappings = Vec::new();
        let mut output = None;
        let mut durability = None;
        let mut compensation = None;

        if input.peek(syn::token::Brace) {
            let content;
            syn::braced!(content in input);
            while !content.is_empty() {
                let key = content.parse::<Ident>()?;
                content.parse::<Token![:]>()?;
                match key.to_string().as_str() {
                    "operation" => { operation = Some(content.parse::<Ident>()?.to_string()); }
                    "output" => { output = Some(content.parse::<LitStr>()?.value()); }
                    "durability" => { durability = Some(content.parse::<Ident>()?.to_string()); }
                    "input" => {
                        let map_content;
                        syn::braced!(map_content in content);
                        mappings = parse_mappings(&map_content)?;
                    }
                    "compensation" => {
                        compensation = Some(parse_compensation(&content)?);
                    }
                    _ => { let _: LitStr = content.parse()?; }
                }
                if content.peek(Token![,]) { content.parse::<Token![,]>()?; }
            }
        }

        Ok(ActivityDef {
            id, kind: ActivityKind::Connector,
            connector_ref: Some(connector_ref), operation, mappings,
            output, durability, compensation,
            response_expr: None, trigger_ref: None,
        })
    } else if lookahead.peek(kw::end_trigger) {
        input.parse::<kw::end_trigger>()?;
        let ref_content;
        syn::parenthesized!(ref_content in input);
        let trigger_ref = ref_content.parse::<LitStr>()?.value();

        let mut response_expr = None;
        if input.peek(syn::token::Brace) {
            let content;
            syn::braced!(content in input);
            while !content.is_empty() {
                let key = content.parse::<Ident>()?;
                content.parse::<Token![:]>()?;
                match key.to_string().as_str() {
                    "response" => {
                        let (lang, src) = parse_expr_value(&content)?;
                        response_expr = Some((lang, src));
                    }
                    _ => { let _: LitStr = content.parse()?; }
                }
                if content.peek(Token![,]) { content.parse::<Token![,]>()?; }
            }
        }

        Ok(ActivityDef {
            id, kind: ActivityKind::EndTrigger,
            connector_ref: None, operation: None, mappings: Vec::new(),
            output: None, durability: None, compensation: None,
            response_expr, trigger_ref: Some(trigger_ref),
        })
    } else if lookahead.peek(kw::transform) {
        input.parse::<kw::transform>()?;
        let mut mappings = Vec::new();
        let mut output = None;
        if input.peek(syn::token::Brace) {
            let content;
            syn::braced!(content in input);
            while !content.is_empty() {
                let key = content.parse::<Ident>()?;
                content.parse::<Token![:]>()?;
                match key.to_string().as_str() {
                    "input" => {
                        let mc;
                        syn::braced!(mc in content);
                        mappings = parse_mappings(&mc)?;
                    }
                    "output" => { output = Some(content.parse::<LitStr>()?.value()); }
                    _ => { let _: LitStr = content.parse()?; }
                }
                if content.peek(Token![,]) { content.parse::<Token![,]>()?; }
            }
        }
        Ok(ActivityDef {
            id, kind: ActivityKind::Transform,
            connector_ref: None, operation: None, mappings,
            output, durability: None, compensation: None,
            response_expr: None, trigger_ref: None,
        })
    } else {
        Err(input.error("expected connector(...), end_trigger(...), or transform"))
    }
}

fn parse_mappings(input: ParseStream) -> Result<Vec<MappingDef>> {
    let mut mappings = Vec::new();
    while !input.is_empty() {
        let target = input.parse::<Ident>()?.to_string();
        input.parse::<Token![:]>()?;
        let (language, source) = parse_expr_value(input)?;
        mappings.push(MappingDef { target, language, source });
        if input.peek(Token![,]) { input.parse::<Token![,]>()?; }
    }
    Ok(mappings)
}

fn parse_expr_value(input: ParseStream) -> Result<(String, String)> {
    let lookahead = input.lookahead1();
    if lookahead.peek(kw::literal) {
        input.parse::<kw::literal>()?;
        let content;
        syn::parenthesized!(content in input);
        let val = content.parse::<LitStr>()?.value();
        Ok(("literal".into(), val))
    } else if lookahead.peek(kw::cel) {
        input.parse::<kw::cel>()?;
        let content;
        syn::parenthesized!(content in input);
        let val = content.parse::<LitStr>()?.value();
        Ok(("vil-expr".into(), val))
    } else if lookahead.peek(kw::spv1) {
        input.parse::<kw::spv1>()?;
        let content;
        syn::parenthesized!(content in input);
        let val = content.parse::<LitStr>()?.value();
        Ok(("spv1".into(), val))
    } else if lookahead.peek(kw::vil_query) {
        input.parse::<kw::vil_query>()?;
        let content;
        syn::parenthesized!(content in input);
        let val = content.parse::<LitStr>()?.value();
        Ok(("vil_query".into(), val))
    } else if lookahead.peek(LitStr) {
        // bare string = literal
        let val = input.parse::<LitStr>()?.value();
        Ok(("literal".into(), val))
    } else {
        Err(input.error("expected literal(...), cel(...), spv1(...), vil_query(...), or string"))
    }
}

fn parse_compensation(input: ParseStream) -> Result<CompensationDef> {
    // compensation: connector("ref") { operation: op, input: { ... } }
    input.parse::<kw::connector>()?;
    let ref_content;
    syn::parenthesized!(ref_content in input);
    let connector_ref = ref_content.parse::<LitStr>()?.value();

    let mut operation = String::new();
    let mut mappings = Vec::new();

    if input.peek(syn::token::Brace) {
        let content;
        syn::braced!(content in input);
        while !content.is_empty() {
            let key = content.parse::<Ident>()?;
            content.parse::<Token![:]>()?;
            match key.to_string().as_str() {
                "operation" => { operation = content.parse::<Ident>()?.to_string(); }
                "input" => {
                    let mc;
                    syn::braced!(mc in content);
                    mappings = parse_mappings(&mc)?;
                }
                _ => { let _: LitStr = content.parse()?; }
            }
            if content.peek(Token![,]) { content.parse::<Token![,]>()?; }
        }
    }

    Ok(CompensationDef { connector_ref, operation, mappings })
}

fn parse_flow(input: ParseStream) -> Result<Vec<String>> {
    let mut nodes = Vec::new();
    nodes.push(input.parse::<Ident>()?.to_string());
    while input.peek(Token![->]) {
        input.parse::<Token![->]>()?;
        nodes.push(input.parse::<Ident>()?.to_string());
    }
    Ok(nodes)
}
