use crate::{
    providers::{DnsRecordInput, DnsRecordType},
    settings::RecordTemplate,
};
use shared::models::node_allocation::NodeAllocation;
use sqlx::types::ipnetwork::IpNetwork;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// Values available to `{placeholder}` expansion in record templates.
pub struct Vars {
    /// Subdomain label, e.g. `mc`.
    pub name: String,
    /// Zone domain, e.g. `example.com`.
    pub domain: String,
    /// Resolved allocation target (ip or hostname).
    pub ip: String,
    /// Allocation port.
    pub port: i32,
    /// Server uuid.
    pub server: uuid::Uuid,
    /// Server name.
    pub server_name: String,
}

impl Vars {
    pub fn fqdn(&self) -> String {
        format!("{}.{}", self.name, self.domain)
    }
}

/// Expands `{name}`, `{domain}`, `{fqdn}`, `{ip}`, `{port}`, `{server}` and
/// `{server_name}` placeholders in a template string.
pub fn render(template: &str, vars: &Vars) -> String {
    template
        .replace("{fqdn}", &vars.fqdn())
        .replace("{name}", &vars.name)
        .replace("{domain}", &vars.domain)
        .replace("{ip}", &vars.ip)
        .replace("{port}", &vars.port.to_string())
        .replace("{server}", &vars.server.to_string())
        .replace("{server_name}", &vars.server_name)
}

/// What an `address` record should point at for a given allocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    Ipv4(Ipv4Addr),
    Ipv6(Ipv6Addr),
    Hostname(String),
}

impl Target {
    pub fn record_type(&self) -> DnsRecordType {
        match self {
            Target::Ipv4(_) => DnsRecordType::A,
            Target::Ipv6(_) => DnsRecordType::AAAA,
            Target::Hostname(_) => DnsRecordType::CNAME,
        }
    }

    pub fn content(&self) -> String {
        match self {
            Target::Ipv4(ip) => ip.to_string(),
            Target::Ipv6(ip) => ip.to_string(),
            Target::Hostname(host) => host.clone(),
        }
    }
}

/// Resolves the DNS target for an allocation: `ip_alias` (trimmed, trailing
/// dots stripped) wins over the raw `ip`. IPs become A/AAAA records, anything
/// else becomes a CNAME. An unspecified IP with no alias is an error.
pub fn resolve_target(allocation: &NodeAllocation) -> Result<Target, anyhow::Error> {
    resolve_target_parts(allocation.ip, allocation.ip_alias.as_deref())
}

fn resolve_target_parts(ip: IpNetwork, ip_alias: Option<&str>) -> Result<Target, anyhow::Error> {
    if let Some(alias) = ip_alias
        .map(str::trim)
        .map(|alias| alias.trim_end_matches('.'))
        .filter(|alias| !alias.is_empty())
    {
        if let Ok(ipv4) = alias.parse::<Ipv4Addr>() {
            return Ok(Target::Ipv4(ipv4));
        }
        if let Ok(ipv6) = alias.parse::<Ipv6Addr>() {
            return Ok(Target::Ipv6(ipv6));
        }
        return Ok(Target::Hostname(alias.to_string()));
    }

    match ip.ip() {
        IpAddr::V4(ipv4) if !ipv4.is_unspecified() => Ok(Target::Ipv4(ipv4)),
        IpAddr::V6(ipv6) if !ipv6.is_unspecified() => Ok(Target::Ipv6(ipv6)),
        _ => Err(
            shared::response::DisplayError::new("allocation has no public address")
                .with_status(axum::http::StatusCode::BAD_REQUEST)
                .into(),
        ),
    }
}

/// Joins a rendered (possibly already qualified) record name onto the zone
/// domain into an FQDN.
fn fqdn_name(rendered: &str, domain: &str) -> String {
    let rendered = rendered.trim_end_matches('.');

    if rendered.is_empty() || rendered == "@" {
        domain.to_string()
    } else if rendered == domain || rendered.ends_with(&format!(".{domain}")) {
        rendered.to_string()
    } else {
        format!("{rendered}.{domain}")
    }
}

/// Renders record templates into provider-ready inputs for the given target.
pub fn render_records(
    templates: &[RecordTemplate],
    vars: &Vars,
    target: &Target,
) -> Result<Vec<DnsRecordInput>, anyhow::Error> {
    let mut records = Vec::with_capacity(templates.len());

    for template in templates {
        records.push(match template {
            RecordTemplate::Address { name, proxied, ttl } => DnsRecordInput {
                record_type: target.record_type(),
                name: fqdn_name(&render(name, vars), &vars.domain),
                content: target.content(),
                ttl: *ttl,
                proxied: *proxied,
                priority: 0,
                weight: 0,
                port: 0,
            },
            RecordTemplate::Srv {
                service,
                protocol,
                priority,
                weight,
                ttl,
            } => DnsRecordInput {
                record_type: DnsRecordType::SRV,
                name: render(&format!("{service}.{protocol}.{{name}}.{{domain}}"), vars),
                content: vars.fqdn(),
                ttl: *ttl,
                proxied: false,
                priority: *priority,
                weight: *weight,
                port: u16::try_from(vars.port).unwrap_or(0),
            },
            RecordTemplate::Custom {
                record_type,
                name,
                content,
                ttl,
            } => {
                if *record_type == DnsRecordType::SRV {
                    anyhow::bail!(
                        "custom records cannot be of type SRV; use the `srv` template kind"
                    );
                }
                DnsRecordInput {
                    record_type: *record_type,
                    name: fqdn_name(&render(name, vars), &vars.domain),
                    content: render(content, vars),
                    ttl: *ttl,
                    proxied: false,
                    priority: 0,
                    weight: 0,
                    port: 0,
                }
            }
        });
    }

    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars() -> Vars {
        Vars {
            name: "mc".to_string(),
            domain: "example.com".to_string(),
            ip: "203.0.113.10".to_string(),
            port: 25565,
            server: uuid::Uuid::nil(),
            server_name: "My Server".to_string(),
        }
    }

    fn allocation(ip: &str) -> IpNetwork {
        ip.parse::<IpNetwork>().unwrap()
    }

    #[test]
    fn render_expands_all_placeholders() {
        let rendered = render(
            "{name}.{domain} {fqdn} {ip} {port} {server} {server_name}",
            &vars(),
        );
        assert_eq!(
            rendered,
            format!(
                "mc.example.com mc.example.com 203.0.113.10 25565 {} My Server",
                uuid::Uuid::nil()
            )
        );
    }

    #[test]
    fn render_leaves_unknown_placeholders() {
        assert_eq!(render("{unknown}", &vars()), "{unknown}");
    }

    #[test]
    fn resolve_target_plain_ipv4() {
        let target = resolve_target_parts(allocation("203.0.113.10/32"), None).unwrap();
        assert_eq!(target, Target::Ipv4("203.0.113.10".parse().unwrap()));
    }

    #[test]
    fn resolve_target_plain_ipv6() {
        let target = resolve_target_parts(allocation("2001:db8::1/128"), None).unwrap();
        assert_eq!(target, Target::Ipv6("2001:db8::1".parse().unwrap()));
    }

    #[test]
    fn resolve_target_alias_hostname() {
        let target =
            resolve_target_parts(allocation("203.0.113.10/32"), Some(" node.example.com. "))
                .unwrap();
        assert_eq!(target, Target::Hostname("node.example.com".to_string()));
    }

    #[test]
    fn resolve_target_alias_ip_wins_over_ip() {
        let target =
            resolve_target_parts(allocation("203.0.113.10/32"), Some("198.51.100.5")).unwrap();
        assert_eq!(target, Target::Ipv4("198.51.100.5".parse().unwrap()));
    }

    #[test]
    fn resolve_target_unspecified_ip_errors() {
        let err = resolve_target_parts(allocation("0.0.0.0/32"), None).unwrap_err();
        assert!(err.to_string().contains("no public address"));
    }

    #[test]
    fn resolve_target_unspecified_ip_with_alias_uses_alias() {
        let target =
            resolve_target_parts(allocation("0.0.0.0/32"), Some("node.example.com")).unwrap();
        assert_eq!(target, Target::Hostname("node.example.com".to_string()));
    }

    #[test]
    fn render_records_address_follows_target_type() {
        let templates = [RecordTemplate::Address {
            name: "{name}".to_string(),
            proxied: true,
            ttl: 0,
        }];
        let records = render_records(
            &templates,
            &vars(),
            &Target::Hostname("node.example.com".to_string()),
        )
        .unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].record_type, DnsRecordType::CNAME);
        assert_eq!(records[0].name, "mc.example.com");
        assert_eq!(records[0].content, "node.example.com");
        assert!(records[0].proxied);
    }

    #[test]
    fn render_records_srv_name_and_target() {
        let templates = [RecordTemplate::Srv {
            service: "_minecraft".to_string(),
            protocol: "_tcp".to_string(),
            priority: 0,
            weight: 5,
            ttl: 0,
        }];
        let records = render_records(
            &templates,
            &vars(),
            &Target::Ipv4("203.0.113.10".parse().unwrap()),
        )
        .unwrap();
        assert_eq!(records[0].record_type, DnsRecordType::SRV);
        assert_eq!(records[0].name, "_minecraft._tcp.mc.example.com");
        assert_eq!(records[0].content, "mc.example.com");
        assert_eq!(records[0].port, 25565);
        assert_eq!(records[0].weight, 5);
    }

    #[test]
    fn render_records_custom_renders_and_qualifies() {
        let templates = [RecordTemplate::Custom {
            record_type: DnsRecordType::TXT,
            name: "_verify.{name}".to_string(),
            content: "server={server}".to_string(),
            ttl: 60,
        }];
        let records = render_records(
            &templates,
            &vars(),
            &Target::Ipv4("203.0.113.10".parse().unwrap()),
        )
        .unwrap();
        assert_eq!(records[0].name, "_verify.mc.example.com");
        assert_eq!(records[0].content, format!("server={}", uuid::Uuid::nil()));
        assert_eq!(records[0].ttl, 60);
    }

    #[test]
    fn render_records_custom_srv_rejected() {
        let templates = [RecordTemplate::Custom {
            record_type: DnsRecordType::SRV,
            name: "x".to_string(),
            content: "y".to_string(),
            ttl: 0,
        }];
        assert!(
            render_records(
                &templates,
                &vars(),
                &Target::Ipv4("203.0.113.10".parse().unwrap())
            )
            .is_err()
        );
    }

    #[test]
    fn fqdn_name_apex_and_already_qualified() {
        assert_eq!(fqdn_name("@", "example.com"), "example.com");
        assert_eq!(fqdn_name("example.com.", "example.com"), "example.com");
        assert_eq!(fqdn_name("a.example.com", "example.com"), "a.example.com");
        assert_eq!(fqdn_name("a", "example.com"), "a.example.com");
    }
}
