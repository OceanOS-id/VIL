use serde::{Deserialize, Serialize};
use proc_macro2::TokenStream;
use quote::{quote, format_ident};

#[cfg(test)]
mod tests;

/// YAML schema definition for VIL Distributed Topology.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopoDef {
    pub name: String,
    pub hosts: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub processes: Vec<String>,
    #[serde(default)]
    pub instances: Vec<InstanceDef>,
    #[serde(default)]
    pub routes: Vec<RouteDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceDef {
    pub name: String,
    pub host: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteDef {
    pub from: String,
    pub to: String,
    pub transfer_mode: String,
    pub transport: Option<String>,
}

/// Parse a YAML string into a `TopoDef`.
pub fn parse_yaml(yaml_content: &str) -> Result<TopoDef, serde_yaml::Error> {
    serde_yaml::from_str(yaml_content)
}

/// Generate a TokenStream containing a `vil_workflow!` macro invocation from a `TopoDef`.
pub fn generate_workflow_macro(topo: &TopoDef) -> TokenStream {
    let workflow_name = &topo.name;

    // 1. Hosts
    let mut hosts_tokens = Vec::new();
    for (name, addr) in &topo.hosts {
        let name_id = format_ident!("{}", name);
        hosts_tokens.push(quote! {
            #name_id: Host(#addr)
        });
    }

    // 2. Processes (without host affinity)
    let processes_tokens = topo.processes.iter().map(|p| format_ident!("{}", p));

    // 3. Instances (with host affinity "@ host_name")
    let mut instances_tokens = Vec::new();
    for inst in &topo.instances {
        let name_id = format_ident!("{}", inst.name);
        if let Some(host) = &inst.host {
            let host_id = format_ident!("{}", host);
            instances_tokens.push(quote! { #name_id @ #host_id });
        } else {
            instances_tokens.push(quote! { #name_id });
        }
    }

    // 4. Routes
    let mut routes_tokens = Vec::new();
    for route in &topo.routes {
        // Expected format: "ProcessA.port_out" -> "ProcessB.port_in"
        let from_parts: Vec<&str> = route.from.split('.').collect();
        let to_parts: Vec<&str> = route.to.split('.').collect();
        
        let src_proc = format_ident!("{}", from_parts[0]);
        let src_port = format_ident!("{}", from_parts[1]);
        let dst_proc = format_ident!("{}", to_parts[0]);
        let dst_port = format_ident!("{}", to_parts[1]);
        
        let t_mode = format_ident!("{}", route.transfer_mode);
        
        if let Some(transport) = &route.transport {
            let transport_id = format_ident!("{}", transport);
            routes_tokens.push(quote! {
                #src_proc.#src_port -> #dst_proc.#dst_port (#t_mode, transport: #transport_id)
            });
        } else {
            routes_tokens.push(quote! {
                #src_proc.#src_port -> #dst_proc.#dst_port (#t_mode)
            });
        }
    }

    quote! {
        vil_workflow! {
            name: #workflow_name,
            hosts: [
                #(#hosts_tokens),*
            ],
            processes: [
                #(#processes_tokens),*
            ],
            instances: [
                #(#instances_tokens),*
            ],
            routes: [
                #(#routes_tokens),*
            ]
        }
    }
}
