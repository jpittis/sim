use plotly::common::color::Rgb;
use plotly::common::{Marker, Mode, Title};
use plotly::layout::{Axis, Layout};
use plotly::{Plot, Scatter};

pub fn chart(with: Vec<f64>, without: Vec<f64>, ylabel: &str, title: &str) -> anyhow::Result<()> {
    let trace1 = Scatter::new(vec![0, 1, 2, 3], with)
        .mode(Mode::LinesMarkers)
        .name("With Token Bucket")
        .marker(Marker::new().color(Rgb::new(219, 64, 82)).size(12));
    let trace2 = Scatter::new(vec![0, 1, 2, 3], without)
        .mode(Mode::LinesMarkers)
        .name("Without Token Bucket")
        .marker(Marker::new().color(Rgb::new(128, 0, 128)).size(12));

    let layout = Layout::new()
        .title(Title::new(title))
        .x_axis(
            Axis::new()
                .title(Title::new("Regions Unavailable"))
                .tick_format(".0f")
                .tick_values(vec![0.0, 1.0, 2.0, 3.0]),
        )
        .y_axis(Axis::new().title(Title::new(ylabel)));

    let mut plot = Plot::new();
    plot.add_trace(trace1);
    plot.add_trace(trace2);
    plot.set_layout(layout);
    plot.show();

    Ok(())
}
