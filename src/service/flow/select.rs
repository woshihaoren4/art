use crate::core::{Ctx, JsonServiceExt, Output, ServiceEntity};
use serde_json::Value;
use std::fmt::Display;
use std::str::FromStr;
use wd_tools::PFErr;

#[derive(Default, Debug)]
pub struct Select {}
#[derive(Default, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct SelectCfg {
    pub conditions: SelectNode,
    pub true_to_nodes: Vec<String>,
    pub false_to_nodes: Vec<String>,
}
#[derive(Default, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SelectNode {
    #[default]
    None,
    Equal(Value, Value),        // =
    NotEqual(Value, Value),     // !=
    Greater(Value, Value),      // >
    GreaterEqual(Value, Value), // >=
    Less(Value, Value),         // <
    LessEqual(Value, Value),    // <=
    Contain(Value, Value),      // 包含
    Empty(Value),
    NonEmpty(Value),
    And(Vec<SelectNode>),
    Or(Vec<SelectNode>),
}

impl Display for SelectNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            serde_json::to_string(&self).unwrap_or(String::new())
        )
    }
}
impl Display for SelectCfg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            serde_json::to_string(&self).unwrap_or(String::new())
        )
    }
}
impl SelectNode {
    pub fn number_value_compare(
        left: Value,
        right: Value,
        opt: SelectNode,
    ) -> anyhow::Result<bool> {
        let mut ty = 1; // 1:i64 2:float
        if left.is_f64() || right.is_f64() || left.is_string() || right.is_string() {
            ty = 2;
        }
        if ty == 1 {
            let left = if let Some(s) = left.as_i64() {
                s
            } else {
                return anyhow::anyhow!("select.number_value_compare:type[{left}] can not as i64")
                    .err();
            };
            let right = if let Some(s) = right.as_i64() {
                s
            } else {
                return anyhow::anyhow!("select.number_value_compare:type[{right}] can not as i64")
                    .err();
            };
            return match opt {
                SelectNode::Greater(_, _) => Ok(left > right),
                SelectNode::GreaterEqual(_, _) => Ok(left >= right),
                SelectNode::Less(_, _) => Ok(left < right),
                SelectNode::LessEqual(_, _) => Ok(left <= right),
                _ => anyhow::anyhow!("select.number_value_compare no support opt[{opt:?}]").err(),
            };
        }
        assert_eq!(ty, 2);
        let left = if left.is_number() {
            if let Some(f) = left.as_f64() {
                f
            } else {
                return anyhow::anyhow!("select.number_value_compare:type[{left}] can not as f64")
                    .err();
            }
        } else if left.is_string() {
            let s = left.as_str().unwrap_or("0");
            f64::from_str(s)?
        } else {
            return anyhow::anyhow!("select.number_value_compare:type[{left}] can not as f64")
                .err();
        };
        let right = if right.is_number() {
            if let Some(f) = right.as_f64() {
                f
            } else {
                return anyhow::anyhow!("select.number_value_compare:type[{right}] can not as f64")
                    .err();
            }
        } else if right.is_string() {
            let s = right.as_str().unwrap_or("0");
            f64::from_str(s)?
        } else {
            return anyhow::anyhow!("select.number_value_compare:type[{right}] can not as f64")
                .err();
        };
        match opt {
            SelectNode::Greater(_, _) => Ok(left > right),
            SelectNode::GreaterEqual(_, _) => Ok(left >= right),
            SelectNode::Less(_, _) => Ok(left < right),
            SelectNode::LessEqual(_, _) => Ok(left <= right),
            _ => anyhow::anyhow!("select.number_value_compare no support opt[{opt:?}]").err(),
        }
    }
    pub fn calc(self) -> anyhow::Result<bool> {
        match self {
            SelectNode::None => Ok(true),
            SelectNode::Equal(a, b) => Ok(a == b),
            SelectNode::NotEqual(a, b) => Ok(a != b),
            SelectNode::Greater(a, b) => {
                Self::number_value_compare(a, b, SelectNode::Greater(Value::Null, Value::Null))
            }
            SelectNode::GreaterEqual(a, b) => {
                Self::number_value_compare(a, b, SelectNode::GreaterEqual(Value::Null, Value::Null))
            }
            SelectNode::Less(a, b) => {
                Self::number_value_compare(a, b, SelectNode::Less(Value::Null, Value::Null))
            }
            SelectNode::LessEqual(a, b) => {
                Self::number_value_compare(a, b, SelectNode::LessEqual(Value::Null, Value::Null))
            }
            SelectNode::Contain(a, b) => {
                match a {
                    Value::String(ref a) => {
                        if let Value::String(ref b) = b {
                            return Ok(a.contains(b));
                        }
                    }
                    Value::Array(ref list) => {
                        for i in list {
                            if i == &b {
                                return Ok(true);
                            }
                        }
                    }
                    _ => {}
                }
                return anyhow::anyhow!("select.Contain[{a:?}] no support type[{b:?}]").err();
            }
            SelectNode::Empty(a) => {
                if !a.is_null() {
                    return Ok(false);
                }
                return Ok(true);
            }
            SelectNode::NonEmpty(a) => {
                if a.is_null() {
                    return Ok(false);
                }

                return Ok(true);
            }
            _ => return anyhow::anyhow!("select.SelectCond[{self:?}] no support calc").err(),
        }
    }
    pub fn generate_result(self) -> anyhow::Result<bool> {
        match self {
            SelectNode::And(list) => {
                if list.len() < 2 {
                    return anyhow::anyhow!("select.SelectNode[and].cond must >= two").err();
                }
                for i in list {
                    if !i.generate_result()? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            SelectNode::Or(list) => {
                if list.len() < 2 {
                    return anyhow::anyhow!("select.SelectNode[or].cond must >= two").err();
                }
                for i in list {
                    if i.generate_result()? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            _ => return self.calc(),
        }
    }
}

#[async_trait::async_trait]
impl JsonServiceExt<SelectCfg, bool> for Select {
    async fn output(&self, out: bool) -> anyhow::Result<Output> {
        Ok(Output::value(out))
    }
    async fn call(&self, ctx: Ctx, cfg: SelectCfg, se: ServiceEntity) -> anyhow::Result<bool> {
        //进行判断
        let res = cfg.conditions.generate_result()?;
        //修改plan
        let node = se.node_name;
        if res {
            ctx.deref_mut_plan(|p| {
                p.set_to(node.as_str(), cfg.true_to_nodes);
            });
        } else {
            ctx.deref_mut_plan(|p| {
                p.set_to(node.as_str(), cfg.false_to_nodes);
            });
        }
        Ok(res)
    }
}

#[cfg(test)]
mod test {
    use crate::service::flow::{SelectCfg, SelectNode};
    use serde_json::Value;

    #[test]
    fn select_config() {
        let cfg = SelectCfg {
            // conditions: SelectNode::And(vec![
            //     SelectNode::Greater(Value::from(123),Value::from(456)),
            //     SelectNode::Equal(Value::from("hello"),Value::from("world")),
            // ]),
            conditions: SelectNode::Greater(Value::from(123), Value::from(456)),
            true_to_nodes: vec!["A".into()],
            false_to_nodes: vec!["B".into()],
        };
        println!("{}", cfg)
    }
}
