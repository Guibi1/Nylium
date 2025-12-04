use std::marker::PhantomData;
use std::time::Duration;

use gpui::{prelude::FluentBuilder, *};
use gpui_component::ActiveTheme;
use gpui_component::chart::AreaChart;
use gpui_component::label::Label;
use nylium_adapter::NyliumServer;
use ringbuffer::{ConstGenericRingBuffer, RingBuffer};

pub struct StatisticsPage<S, C, G>
where
    C: Copy,
    G: Copy,
    S: NyliumServer<C, G>,
{
    data_points: ConstGenericRingBuffer<DataPoint, 32>,
    _phantom: PhantomData<(S, C, G)>,
}

impl<S, C, G> StatisticsPage<S, C, G>
where
    C: Copy + 'static,
    G: Copy + 'static,
    S: NyliumServer<C, G>,
{
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let (tx, rx) = smol::channel::bounded::<DataPoint>(1);

        cx.background_spawn(async move {
            let mut sys = sysinfo::System::new_all();
            let pid = sysinfo::get_current_pid().unwrap();

            loop {
                sys.refresh_processes_specifics(
                    sysinfo::ProcessesToUpdate::Some(&[pid]),
                    false,
                    sysinfo::ProcessRefreshKind::nothing()
                        .with_cpu()
                        .with_memory(),
                );

                if let Some(proc) = sys.process(pid) {
                    let memory = proc.memory() as f64 / 1_000_000.;
                    let cpu = proc.cpu_usage() as f64 / sys.cpus().len() as f64;
                    tx.try_send(DataPoint { memory, cpu }).ok();
                }

                Timer::after(Duration::from_secs(1)).await;
            }
        })
        .detach();

        cx.spawn(async move |this, cx| {
            while let Ok(data_point) = rx.recv().await {
                let _ = this.update(cx, |this, cx| {
                    this.data_points.enqueue(data_point);
                    cx.notify();
                });
            }
        })
        .detach();

        Self {
            data_points: ConstGenericRingBuffer::from(
                &[DataPoint {
                    memory: 0.0,
                    cpu: 0.0,
                }; 32],
            ),
            _phantom: PhantomData,
        }
    }
}

impl<S, C, G> Render for StatisticsPage<S, C, G>
where
    C: Copy + 'static,
    G: Copy + 'static,
    S: NyliumServer<C, G>,
{
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let data = self
            .data_points
            .iter()
            .skip(2)
            .copied()
            .enumerate()
            .collect::<Box<_>>();

        div()
            .flex_grow()
            .px_4()
            .pt_4()
            .relative()
            .flex()
            .flex_col()
            .gap_2()
            .child(Label::new("System Statistics").text_xl())
            .when_some(self.data_points.back(), |this, latest| {
                this.child(
                    div()
                        .px_4()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(div().size_2().bg(cx.theme().chart_2))
                                .child(Label::new(format!("CPU usage: {:.1}%", latest.cpu))),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(div().size_2().bg(cx.theme().chart_1))
                                .child(Label::new(format!("RAM usage: {:.1}MB", latest.memory))),
                        ),
                )
            })
            .child(
                div().flex_grow().p_4().child(
                    AreaChart::new(data)
                        .x(|(i, _)| {
                            if *i == 29 {
                                SharedString::from("now")
                            } else {
                                format!("{}s ago", 29 - i).into()
                            }
                        })
                        .y(|(_, d)| d.memory) // First series
                        .stroke(cx.theme().chart_1)
                        .fill(cx.theme().chart_1.opacity(0.1))
                        .y(|(_, d)| d.cpu) // Second series
                        .stroke(cx.theme().chart_2)
                        .fill(cx.theme().chart_2.opacity(0.1))
                        .linear()
                        .tick_margin(5),
                ),
            )
    }
}

#[derive(Clone, Copy)]
struct DataPoint {
    memory: f64,
    cpu: f64,
}
