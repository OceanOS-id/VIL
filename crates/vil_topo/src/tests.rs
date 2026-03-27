#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn test_parse_yaml() {
        let yaml = r#"
name: DistributedSensorPipeline
hosts:
  node_edge: "192.168.1.10:9000"
  node_core: "10.0.0.5:9000"
instances:
  - name: RemoteIngress
    host: node_edge
  - name: RemoteProcessor
    host: node_core
routes:
  - from: RemoteIngress.data_out
    to: RemoteProcessor.data_in
    transfer_mode: LoanWrite
    transport: RDMA
"#;
        let topo = parse_yaml(yaml).expect("Failed to parse YAML");
        assert_eq!(topo.name, "DistributedSensorPipeline");
        assert_eq!(topo.hosts.len(), 2);
        assert_eq!(topo.instances.len(), 2);
        assert_eq!(topo.routes.len(), 1);
        
        let r1 = &topo.routes[0];
        assert_eq!(r1.from, "RemoteIngress.data_out");
        assert_eq!(r1.transfer_mode, "LoanWrite");
        assert_eq!(r1.transport, Some("RDMA".to_string()));
    }

    #[test]
    fn test_generate_macro() {
        let yaml = r#"
name: SimpleTopo
hosts: {}
processes:
  - Writer
  - Reader
routes:
  - from: Writer.out
    to: Reader.in
    transfer_mode: Copy
"#;
        let topo = parse_yaml(yaml).unwrap();
        let tokens = generate_workflow_macro(&topo);
        let code = tokens.to_string();
        
        println!("MACRO OUTPUT:\n{}", code);
        
        assert!(code.contains("vil_workflow !"));
        assert!(code.contains("name : \"SimpleTopo\""));
        assert!(code.contains("processes : [Writer , Reader]"));
        assert!(code.contains("routes : [Writer . out -> Reader . in (Copy)]"));
    }
}
