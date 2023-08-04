use super::*;

pub struct Net {
    opts: GeneralOpt,
    fields: Vec<NetField>,
}

impl Net {
    pub fn new(
        opts: &GeneralOpt,
        fields: Vec<NetField>
    ) -> Self {
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
        let mut queues = Vec::new();
        for (_, nic) in &model.net.nic {
            for queue in &nic.queue {
                queues.push(queue);
            }
        }

        // Return if we filtered everything.
        if queues.is_empty() {
            return Ok(IterExecResult::Skip);
        }

        let mut json_output = json!([]);

        queues
            .into_iter()
            .map(|queue| {
                match self.opts.output_format {
                    Some(OutputFormat::Raw) | None => write!(
                        output,
                        "{}",
                        print::dump_raw(
                            &self.fields,
                            ctx,
                            queue,
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
                            queue,
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
                            queue,
                            *round,
                            self.opts.disable_title,
                            self.opts.raw
                        )
                    )?,
                    Some(OutputFormat::KeyVal) => write!(
                        output,
                        "{}",
                        print::dump_kv(&self.fields, ctx, queue, self.opts.raw)
                    )?,
                    Some(OutputFormat::Json) => {
                        let par = print::dump_json(&self.fields, ctx, queue, self.opts.raw);
                        json_output.as_array_mut().unwrap().push(par);
                    }
                    Some(OutputFormat::OpenMetrics) => write!(
                        output,
                        "{}",
                        print::dump_openmetrics(&self.fields, ctx, queue)
                    )?,
                }
                *round += 1;
                Ok(())
            })
            .collect::<Result<Vec<_>>>()?;

        match (self.opts.output_format, comma_flag) {
            (Some(OutputFormat::Json), true) => write!(output, ",{}", json_output)?,
            (Some(OutputFormat::Json), false) => write!(output, "{}", json_output)?,
            (Some(OutputFormat::OpenMetrics), _) => (),
            _ => write!(output, "\n")?,
        };

        Ok(IterExecResult::Success)
    }
}
