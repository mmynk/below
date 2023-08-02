use super::*;

pub struct Net {
    opts: GeneralOpt,
    fields: Vec<NetField>,
}

impl Net {
    pub fn new(opts: &GeneralOpt, fields: Vec<NetField>) -> Self {
        Self {
            opts: opts.to_owned(),
            fields,
        }
    }
}

impl Dumper for Net {
    fn dump_model(
        &self,
        ctx: &CommonFieldContext,
        model: &model::Model,
        output: &mut dyn Write,
        round: &mut usize,
        comma_flag: bool,
    ) -> Result<IterExecResult> {
        match self.opts.output_format {
            Some(OutputFormat::Raw) | None => write!(
                output,
                "{}",
                print::dump_raw(
                    &self.fields,
                    ctx,
                    &model.net,
                    *round,
                    self.opts.repeat_title,
                    self.opts.disable_title,
                    self.opts.raw
                )
            )?,
            Some(OutputFormat::Csv) => write!(
                output,
                "{}",
                print::dump_csv(
                    &self.fields,
                    ctx,
                    &model.net,
                    *round,
                    self.opts.disable_title,
                    self.opts.raw
                )
            )?,
            Some(OutputFormat::Tsv) => write!(
                output,
                "{}",
                print::dump_tsv(
                    &self.fields,
                    ctx,
                    &model.net,
                    *round,
                    self.opts.disable_title,
                    self.opts.raw
                )
            )?,
            Some(OutputFormat::KeyVal) => write!(
                output,
                "{}",
                print::dump_kv(&self.fields, ctx, &model.net, self.opts.raw)
            )?,
            Some(OutputFormat::Json) => {
                let par = print::dump_json(&self.fields, ctx, &model.net, self.opts.raw);
                if comma_flag {
                    write!(output, ",{}", par.to_string())?;
                } else {
                    write!(output, "{}", par.to_string())?;
                }
            }
            Some(OutputFormat::OpenMetrics) => write!(
                output,
                "{}",
                print::dump_openmetrics(&self.fields, ctx, &model.net)
            )?,
        };

        *round += 1;

        Ok(IterExecResult::Success)
    }
}
