use crate::core::{Ctx, JsonServiceExt, Output, ServiceEntity};
use serde_json::Value;
use std::fmt::Display;
use std::str::FromStr;
use wd_tools::PFErr;

#[derive(Default, Debug)]
pub struct Select {}
#[derive(Default, Debug, serde::Serialize, serde::Deserialize)]
pub struct SelectCfg {
    pub conditions: SelectTree,
    pub true_to_nodes: Vec<String>,
    pub false_to_nodes: Vec<String>,
}
#[derive(Default, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SelectCond {
    #[default]
    None,
    Equal,    // =
    NotEqual, // !=
    Greater,  // >
    Less,     // <
    Empty,
    NonEmpty,
    And,
    Or,
}
#[derive(Default, Debug, serde::Serialize, serde::Deserialize)]
pub struct SelectNode {
    pub cond: SelectCond,
    pub sub: Vec<SelectTree>,
}
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SelectTree {
    Value(Value),
    Cond(SelectNode),
}
impl Default for SelectTree {
    fn default() -> Self {
        Self::Cond(SelectNode::default())
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
impl SelectCond {
    pub fn number_value_compare(
        left: Value,
        right: Value,
        opt: SelectCond,
    ) -> anyhow::Result<bool> {
        let mut ty = 1; // 1:i64 2:float
        if left.is_f64() || right.is_f64() || left.is_string() || right.is_string() {
            ty = 2;
        }
        if ty == 1 {
            let left = if let Some(s) = left.as_i64() {
                s
            } else {
                return anyhow::anyhow!("type[{left}] can not as i64").err();
            };
            let right = if let Some(s) = right.as_i64() {
                s
            } else {
                return anyhow::anyhow!("type[{right}] can not as i64").err();
            };
            return match opt {
                SelectCond::Greater => Ok(left > right),
                SelectCond::Less => Ok(left < right),
                _ => anyhow::anyhow!("number_value_compare no support opt[{opt:?}]").err(),
            };
        }
        assert_eq!(ty, 2);
        let left = if left.is_number() {
            if let Some(f) = left.as_f64() {
                f
            } else {
                return anyhow::anyhow!("type[{left}] can not as f64").err();
            }
        } else if left.is_string() {
            let s = left.as_str().unwrap_or("0");
            f64::from_str(s)?
        } else {
            return anyhow::anyhow!("type[{left}] can not as f64").err();
        };
        let right = if right.is_number() {
            if let Some(f) = right.as_f64() {
                f
            } else {
                return anyhow::anyhow!("type[{right}] can not as f64").err();
            }
        } else if right.is_string() {
            let s = right.as_str().unwrap_or("0");
            f64::from_str(s)?
        } else {
            return anyhow::anyhow!("type[{right}] can not as f64").err();
        };
        match opt {
            SelectCond::Greater => Ok(left > right),
            SelectCond::Less => Ok(left < right),
            _ => anyhow::anyhow!("number_value_compare no support opt[{opt:?}]").err(),
        }
    }
    pub fn calc(self, mut args: Vec<Value>) -> anyhow::Result<bool> {
        match self {
            SelectCond::None => Ok(false),
            SelectCond::Equal => {
                if args.len() != 2 {
                    return anyhow::anyhow!("SelectCond:[Equal] must have two var").err();
                }
                let res = args[0] == args[1];
                Ok(res)
            }
            SelectCond::NotEqual => {
                if args.len() != 2 {
                    return anyhow::anyhow!("SelectCond:[NotEqual] must have two var").err();
                }
                let res = args[0] == args[1];
                Ok(!res)
            }
            SelectCond::Greater => {
                if args.len() != 2 {
                    return anyhow::anyhow!("SelectCond:[Greater] must have two var").err();
                }
                Self::number_value_compare(args.remove(0), args.remove(0), self)
            }
            SelectCond::Less => {
                if args.len() != 2 {
                    return anyhow::anyhow!("SelectCond:[Less] must have two var").err();
                }
                Self::number_value_compare(args.remove(0), args.remove(0), self)
            }
            SelectCond::Empty => {
                for i in args {
                    if !i.is_null() {
                        return Ok(false);
                    }
                }
                return Ok(true);
            }
            SelectCond::NonEmpty => {
                if args.is_empty() {
                    return Ok(false);
                }
                for i in args {
                    if i.is_null() {
                        return Ok(false);
                    }
                }
                return Ok(true);
            }
            _ => return anyhow::anyhow!("SelectCond[{self:?}] no support calc").err(),
        }
    }
}
impl SelectTree {
    pub fn get_value(self) -> anyhow::Result<Value> {
        match self {
            SelectTree::Value(val) => Ok(val),
            SelectTree::Cond(_cond) => anyhow::anyhow!("[SelectTree] this is not value").err(),
        }
    }
    pub fn generate_result(self) -> anyhow::Result<Value> {
        match self {
            SelectTree::Value(val) => Ok(val),
            SelectTree::Cond(cond) => match cond.cond {
                SelectCond::And => {
                    if cond.sub.len() < 2 {
                        return anyhow::anyhow!("[SelectTree] conditions[and] must >= two").err();
                    }
                    for i in cond.sub {
                        if let Value::Bool(b) = i.generate_result()? {
                            if !b {
                                return Ok(Value::from(false));
                            }
                        }
                    }
                    Ok(Value::from(true))
                }
                SelectCond::Or => {
                    if cond.sub.len() < 2 {
                        return anyhow::anyhow!("[SelectTree] conditions[or] must >= two").err();
                    }
                    for i in cond.sub {
                        if let Value::Bool(b) = i.generate_result()? {
                            if b {
                                return Ok(Value::from(true));
                            }
                        }
                    }
                    Ok(Value::from(false))
                }
                _ => {
                    let mut args = Vec::with_capacity(cond.sub.len());
                    for i in cond.sub {
                        args.push(i.get_value()?);
                    }
                    let res = cond.cond.calc(args)?;
                    Ok(Value::from(res))
                }
            },
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
        let res = if let Value::Bool(b) = res {
            b
        } else {
            //非判断值，都为true
            true
        };
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
    use crate::service::flow::{SelectCfg, SelectCond, SelectNode, SelectTree};
    use serde_json::Value;

    #[test]
    fn select_config() {
        let cfg = SelectCfg {
            conditions: SelectTree::Cond(SelectNode {
                cond: SelectCond::And,
                sub: vec![SelectTree::Cond(SelectNode {
                    cond: SelectCond::Equal,
                    sub: vec![
                        SelectTree::Value(Value::from(123)),
                        SelectTree::Value(Value::from("456")),
                    ],
                })],
            }),
            true_to_nodes: vec!["A".into()],
            false_to_nodes: vec!["B".into()],
        };
        println!("{}", cfg)
    }
}
