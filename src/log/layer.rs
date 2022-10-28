pub struct Layer {
    last_span: std::sync::atomic::AtomicU64,
}

impl Layer {
    #[must_use]
    pub fn new() -> Self {
        Self {
            last_span: std::sync::atomic::AtomicU64::new(0),
        }
    }
}

impl Default for Layer {
    fn default() -> Self {
        Self::new()
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

impl tracing_subscriber::Layer<tracing_subscriber::Registry> for Layer {
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
        }
    }

    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        ctx: tracing_subscriber::layer::Context<'_, tracing_subscriber::Registry>,
    ) {
        let mut stdout = std::io::stdout().lock();

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
        let prev_span = ctx
            .span(&id)
            .and_then(|s| s.parent())
            .map_or(0, |p| p.id().into_u64());
        self.last_span
            .store(prev_span, std::sync::atomic::Ordering::Relaxed);
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

                std::mem::drop(write!(
                    out,
                    "[;2m[{timestamp}][m {indent:>0$}[m{path}::{name}{arrow} [37m[{id:04x}][36m",
                    depth * 2,
                    timestamp = info.date_time.format("%Y-%m-%d %H:%M:%S"),
                    indent = "",
                    path = span.metadata().module_path().unwrap_or(""),
                    name = span.name(),
                    arrow = if new { " " } else { "[93m^" },
                    id = info.id,
                ));
                for (k, v) in &info.records {
                    std::mem::drop(write!(out, " [2m{k}: [22m{v}"));
                }
                std::mem::drop(writeln!(out, "[m"));
            }
        } else {
            std::mem::drop(writeln!(out, "[31mFailed to read span info[m"));
        }
    }
}

fn print_event(out: &mut impl std::io::Write, event: &tracing::Event<'_>, depth: usize) {
    struct Visitor<'a, W>(&'a mut W, Option<String>, bool);

    impl<W: std::io::Write> tracing_subscriber::field::Visit for Visitor<'_, W> {
        fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
            if field.name() == "message" {
                self.1 = Some(format!("{value:?}"));
            } else {
                self.2 = true;
                std::mem::drop(write!(self.0, " [2m{field}: [22m{value:?}"));
            }
        }
    }

    std::mem::drop(write!(
        out,
        "[;2m[{timestamp}][m {indent:>0$}{level}[m",
        depth * 2,
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

    let mut visitor = Visitor(out, None, false);
    event.record(&mut visitor);
    if let Some(message) = visitor.1 {
        if visitor.2 {
            std::mem::drop(writeln!(out, " :: {message}"));
        } else {
            std::mem::drop(writeln!(out, " {message}"));
        }
    } else {
        std::mem::drop(writeln!(out));
    }
}
