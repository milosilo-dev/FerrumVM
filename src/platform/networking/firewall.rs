use nftables::{
    batch::Batch,
    expr::{CT, Expression, Meta, MetaKey, NamedExpression, Payload, PayloadField, Prefix},
    helper,
    schema::{Chain, NfListObject, Rule, Table},
    stmt::{Match, Operator, Statement},
    types::{NfChainPolicy, NfChainType, NfFamily, NfHook},
};

pub fn setup_ferrumvm_firewall(iface_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut batch = Batch::new();

    // table ip ferrumvm
    batch.add(NfListObject::Table(Table {
        family: NfFamily::IP,
        name: "ferrumvm".into(),
        ..Default::default()
    }));

    // chain postrouting {
    //     type nat hook postrouting priority srcnat;
    //     policy accept;
    // }
    batch.add(NfListObject::Chain(Chain {
        family: NfFamily::IP,
        table: "ferrumvm".into(),
        name: "postrouting".into(),
        _type: Some(NfChainType::NAT),
        hook: Some(NfHook::Postrouting),
        prio: Some(100),
        policy: Some(NfChainPolicy::Accept),
        ..Default::default()
    }));

    // chain forward {
    //     type filter hook forward priority filter;
    //     policy accept;
    // }
    batch.add(NfListObject::Chain(Chain {
        family: NfFamily::IP,
        table: "ferrumvm".into(),
        name: "forward".into(),
        _type: Some(NfChainType::Filter),
        hook: Some(NfHook::Forward),
        prio: Some(0),
        policy: Some(NfChainPolicy::Accept),
        ..Default::default()
    }));

    // oifname != "tap24" ip saddr 10.0.0.0/24 masquerade
    batch.add(NfListObject::Rule(Rule {
        family: NfFamily::IP,
        table: "ferrumvm".into(),
        chain: "postrouting".into(),
        expr: vec![
            // oifname != "tap24"
            Statement::Match(Match {
                left: Expression::Named(NamedExpression::Meta(Meta {
                    key: MetaKey::Oifname,
                })),
                op: Operator::NEQ,
                right: Expression::String(iface_name.into()),
            }),
            // ip saddr 10.0.0.0/24
            Statement::Match(Match {
                left: Expression::Named(NamedExpression::Payload(Payload::PayloadField(
                    PayloadField {
                        protocol: "ip".into(),
                        field: "saddr".into(),
                    },
                ))),
                op: Operator::EQ,
                right: Expression::Named(NamedExpression::Prefix(Prefix {
                    addr: Box::new(Expression::String("10.0.0.0".into())),
                    len: 24,
                })),
            }),
            // masquerade
            Statement::Masquerade(None),
        ]
        .into(),
        handle: None,
        index: None,
        comment: None,
    }));

    // iifname "tap24" accept
    batch.add(NfListObject::Rule(Rule {
        family: NfFamily::IP,
        table: "ferrumvm".into(),
        chain: "forward".into(),
        expr: vec![
            Statement::Match(Match {
                left: Expression::Named(NamedExpression::Meta(Meta {
                    key: MetaKey::Iifname,
                })),
                op: Operator::EQ,
                right: Expression::String(iface_name.into()),
            }),
            Statement::Accept(None),
        ]
        .into(),
        handle: None,
        index: None,
        comment: None,
    }));

    // oifname "tap24" ct state established,related accept
    batch.add(NfListObject::Rule(Rule {
        family: NfFamily::IP,
        table: "ferrumvm".into(),
        chain: "forward".into(),
        expr: vec![
            // oifname "tap24"
            Statement::Match(Match {
                left: Expression::Named(NamedExpression::Meta(Meta {
                    key: MetaKey::Oifname,
                })),
                op: Operator::EQ,
                right: Expression::String(iface_name.into()),
            }),
            // ct state established,related
            Statement::Match(Match {
                left: Expression::Named(NamedExpression::CT(CT {
                    key: "state".into(),
                    family: None,
                    dir: None,
                })),
                op: Operator::IN,
                right: Expression::List(vec![
                    Expression::String("established".into()),
                    Expression::String("related".into()),
                ]),
            }),
            Statement::Accept(None),
        ]
        .into(),
        handle: None,
        index: None,
        comment: None,
    }));

    helper::apply_ruleset(&batch.to_nftables())?;

    Ok(())
}
