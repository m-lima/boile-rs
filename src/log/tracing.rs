#[must_use]
pub fn layer<O: super::Output>(
    output: O,
) -> impl tracing_subscriber::Layer<tracing_subscriber::Registry> {
    Layer::<O>::new(output)
}

struct Layer<O: super::Output> {
    last_span: std::sync::atomic::AtomicU64,
    output: O,
}

impl<O: super::Output> Layer<O> {
    #[must_use]
    pub fn new(output: O) -> Self {
        Self {
            last_span: std::sync::atomic::AtomicU64::new(0),
            output,
        }
    }
}

struct SpanInfo {
    id: u16,
    date_time: chrono::DateTime<chrono::Utc>,
    records: Vec<(&'static str, String)>,
    new: std::sync::atomic::AtomicBool,
}

impl SpanInfo {
    fn new(attrs: &tracing::span::Attributes<'_>) -> Self {
        struct Visistor(Vec<(&'static str, String)>);

        impl tracing_subscriber::field::Visit for Visistor {
            fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
                self.0.push((field.name(), format!("{value:?}")));
            }
        }

        let mut visitor = Visistor(Vec::with_capacity(attrs.fields().len()));
        attrs.record(&mut visitor);

        Self {
            id: rand::random(),
            date_time: chrono::Utc::now(),
            records: visitor.0,
            new: std::sync::atomic::AtomicBool::new(true),
        }
    }
}

impl<O: super::Output> tracing_subscriber::Layer<tracing_subscriber::Registry> for Layer<O> {
    fn on_new_span(
        &self,
        attrs: &tracing::span::Attributes<'_>,
        id: &tracing::span::Id,
        ctx: tracing_subscriber::layer::Context<'_, tracing_subscriber::Registry>,
    ) {
        if let Some(span) = ctx.span(id) {
            if span.extensions().get::<SpanInfo>().is_none() {
                span.extensions_mut().insert(SpanInfo::new(attrs));
            }

            #[cfg(feature = "log-spans")]
            {
                let mut stdout = self.output.lock();

                let depth = ctx.span_scope(id).map_or(0, std::iter::Iterator::count);
                let last_span = self.last_span.load(std::sync::atomic::Ordering::Relaxed);

                print_span(
                    &mut stdout,
                    last_span,
                    depth.max(1) - 1,
                    Some(span).as_ref(),
                );

                self.last_span
                    .store(id.into_u64(), std::sync::atomic::Ordering::Relaxed);
            }
        }
    }

    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        ctx: tracing_subscriber::layer::Context<'_, tracing_subscriber::Registry>,
    ) {
        let mut stdout = self.output.lock();

        let depth = ctx.event_scope(event).map_or(0, std::iter::Iterator::count);
        let current_span = ctx.current_span().id().and_then(|id| ctx.span(id));
        let last_span = self.last_span.load(std::sync::atomic::Ordering::Relaxed);

        print_span(
            &mut stdout,
            last_span,
            depth.max(1) - 1,
            current_span.as_ref(),
        );

        self.last_span.store(
            current_span.as_ref().map_or(0, |s| s.id().into_u64()),
            std::sync::atomic::Ordering::Relaxed,
        );

        print_event(&mut stdout, event, depth);
    }

    fn on_close(
        &self,
        id: tracing::span::Id,
        ctx: tracing_subscriber::layer::Context<'_, tracing_subscriber::Registry>,
    ) {
        let lock = self.output.lock();
        let last_span = self.last_span.load(std::sync::atomic::Ordering::Relaxed);

        if last_span == id.into_u64() {
            let prev_span = ctx
                .span(&id)
                .and_then(|s| s.parent())
                .map_or(0, |p| p.id().into_u64());
            self.last_span
                .store(prev_span, std::sync::atomic::Ordering::Relaxed);
        }
        drop(lock);
    }
}

fn print_span(
    out: &mut impl std::io::Write,
    last_span: u64,
    depth: usize,
    span: Option<&tracing_subscriber::registry::SpanRef<'_, tracing_subscriber::Registry>>,
) {
    if let Some(span) = span {
        if let Some(info) = span.extensions().get::<SpanInfo>() {
            let new = info.new.swap(false, std::sync::atomic::Ordering::Relaxed);

            if span.id().into_u64() != last_span || new {
                print_span(out, last_span, depth.max(1) - 1, span.parent().as_ref());

                let path = span.metadata().target();
                let name = span.name();
                let div = if path.is_empty() || name.is_empty() {
                    ""
                } else {
                    "::"
                };

                drop(write!(
                    out,
                    "[;2m[{timestamp}][m {indent:>0$}[m{path}{div}[37m{name}",
                    depth * 2,
                    timestamp = info.date_time.format("%Y-%m-%d %H:%M:%S"),
                    indent = "",
                ));

                for (k, v) in &info.records {
                    if *k == "message" {
                        let space = if path.is_empty() && name.is_empty() {
                            ""
                        } else {
                            " "
                        };
                        drop(write!(out, "[m{space}{v}"));
                        break;
                    }
                }

                drop(write!(
                    out,
                    "{arrow} [37m[{id:04x}][36m",
                    arrow = if new { " " } else { "[93m^" },
                    id = info.id,
                ));

                for (k, v) in &info.records {
                    if *k == "message" {
                        continue;
                    }

                    #[cfg(feature = "log-multi-line")]
                    drop(write!(
                        out,
                        "\n{indent:>0$}- [2m{k}: [22m{v}",
                        depth * 2 + 22,
                        indent = ""
                    ));
                    #[cfg(not(feature = "log-multi-line"))]
                    drop(write!(out, " [2m{k}: [22m{v}"));
                }
                drop(writeln!(out, "[m"));
            }
        } else {
            drop(writeln!(out, "[31mFailed to read span info[m"));
        }
    }
}

fn print_event(out: &mut impl std::io::Write, event: &tracing::Event<'_>, depth: usize) {
    struct Messenger<'w, W>(&'w mut W);
    impl<W: std::io::Write> tracing_subscriber::field::Visit for Messenger<'_, W> {
        fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
            if field.name() == "message" {
                drop(write!(self.0, " {value:?}"));
            }
        }
    }

    struct Fielder<'w, W>(&'w mut W, #[cfg(feature = "log-multi-line")] usize);
    impl<W: std::io::Write> tracing_subscriber::field::Visit for Fielder<'_, W> {
        fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
            if field.name() != "message" {
                #[cfg(feature = "log-multi-line")]
                drop(write!(
                    self.0,
                    "\n{indent:>0$}- [36;2m{field}: [22m{value:?}",
                    self.1 + 22,
                    indent = ""
                ));
                #[cfg(not(feature = "log-multi-line"))]
                drop(write!(self.0, " [36;2m{field}: [22m{value:?}"));
            }
        }
    }

    let depth = depth * 2;
    drop(write!(
        out,
        "[;2m[{timestamp}][m {indent:>0$}{level}[m",
        depth,
        timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"),
        indent = "",
        level = match *event.metadata().level() {
            tracing::Level::TRACE => {
                "[94mTRACE"
            }
            tracing::Level::DEBUG => {
                "[34mDEBUG"
            }
            tracing::Level::INFO => {
                "[32mINFO"
            }
            tracing::Level::WARN => {
                "[33mWARN"
            }
            tracing::Level::ERROR => {
                "[31mERROR"
            }
        }
    ));

    event.record(&mut Messenger(out));
    event.record(&mut Fielder(
        out,
        #[cfg(feature = "log-multi-line")]
        depth,
    ));
    drop(writeln!(out, "[m"));
}
