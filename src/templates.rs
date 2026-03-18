use minijinja::{Environment, Value};
use std::sync::Arc;

pub struct Templates {
  env: Arc<Environment<'static>>,
}

impl Templates {
  pub fn new() -> anyhow::Result<Self> {
    let mut env = Environment::new();
    env.set_loader(minijinja::path_loader("templates"));

    env.add_filter("default", |value: Value, default: Value| -> Value {
      if value.is_undefined() || value.is_none() {
        default
      } else {
        value
      }
    });

    Ok(Self { env: Arc::new(env) })
  }

  pub fn render(&self, name: &str, ctx: Value) -> Result<String, minijinja::Error> {
    let tmpl = self.env.get_template(name)?;
    tmpl.render(ctx)
  }
}
