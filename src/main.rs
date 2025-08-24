use dioxus::prelude::*;

mod components;
mod data;

use components::{SnapshotTimelineTab, TableInfoTab};
use data::generate_sample_table;

#[derive(Debug, Clone, PartialEq)]
enum ActiveTab {
    TableInfo,
    SnapshotHistory,
}

fn main() {
    dioxus_logger::init(log::LevelFilter::Info).expect("failed to init logger");
    launch(App);
}

fn App() -> Element {
    let mut active_tab = use_signal(|| ActiveTab::TableInfo);
    let table_data = use_signal(generate_sample_table);

    rsx! {
        div {
            class: "min-h-screen bg-gray-100",
            // Header
            header {
                class: "bg-white shadow-sm border-b",
                div {
                    class: "max-w-7xl mx-auto px-4 sm:px-6 lg:px-8",
                    div {
                        class: "flex justify-between items-center py-6",
                        div {
                            class: "flex items-center",
                            h1 {
                                class: "text-3xl font-bold text-gray-900",
                                "🧊 Iceberg Table Viewer"
                            }
                        }
                        div {
                            class: "text-sm text-gray-500",
                            "Table: {table_data().name}"
                        }
                    }
                }
            }

            // Tab Navigation
            div {
                class: "bg-white border-b",
                div {
                    class: "max-w-7xl mx-auto px-4 sm:px-6 lg:px-8",
                    nav {
                        class: "flex space-x-8",
                        button {
                            class: format!(
                                "py-4 px-1 border-b-2 font-medium text-sm transition-colors {}",
                                if *active_tab.read() == ActiveTab::TableInfo {
                                    "border-blue-500 text-blue-600"
                                } else {
                                    "border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300"
                                }
                            ),
                            onclick: move |_| active_tab.set(ActiveTab::TableInfo),
                            "📋 Table Information"
                        }
                        button {
                            class: format!(
                                "py-4 px-1 border-b-2 font-medium text-sm transition-colors {}",
                                if *active_tab.read() == ActiveTab::SnapshotHistory {
                                    "border-blue-500 text-blue-600"
                                } else {
                                    "border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300"
                                }
                            ),
                            onclick: move |_| active_tab.set(ActiveTab::SnapshotHistory),
                            "📈 Snapshot History"
                        }
                    }
                }
            }

            // Tab Content
            main {
                class: "max-w-7xl mx-auto py-6 px-4 sm:px-6 lg:px-8",
                match *active_tab.read() {
                    ActiveTab::TableInfo => rsx! {
                        TableInfoTab { table: table_data() }
                    },
                    ActiveTab::SnapshotHistory => rsx! {
                        SnapshotTimelineTab { table: table_data() }
                    },
                }
            }
        }

        // Include Tailwind CSS
        //style {
        //    r#"
        //    @import url('https://cdn.jsdelivr.net/npm/tailwindcss@2.2.19/dist/tailwind.min.css');
        //
        //    .timeline-item {
        //        position: relative;
        //        padding-left: 2rem;
        //        margin-bottom: 2rem;
        //    }
        //
        //    .timeline-item::before {
        //        content: '';
        //        position: absolute;
        //        left: 0.5rem;
        //        top: 0.5rem;
        //        width: 0.75rem;
        //        height: 0.75rem;
        //        background-color: #3b82f6;
        //        border-radius: 50%;
        //        border: 2px solid white;
        //        box-shadow: 0 0 0 2px #3b82f6;
        //    }
        //
        //    .timeline-item::after {
        //        content: '';
        //        position: absolute;
        //        left: 0.875rem;
        //        top: 1.25rem;
        //        width: 2px;
        //        height: calc(100% + 1rem);
        //        background-color: #e5e7eb;
        //    }
        //
        //    .timeline-item:last-child::after {
        //        display: none;
        //    }
        //    "#
        //}
    }
}
