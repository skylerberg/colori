use eframe::egui;
use egui_snarl::ui::{PinInfo, SnarlViewer, SnarlStyle};
use egui_snarl::{InPin, InPinId, NodeId, OutPin, OutPinId, Snarl};

use colori_core::scoring::diff_eval::{DiffEvalParams, MLP_INPUT_SIZE, MLP_HIDDEN_SIZE, MLP_HIDDEN2_SIZE};

// ── Parameter index constants (mirrored from diff_eval.rs) ──
// New architecture: pure MLP, no module params. MLP weights start at index 0.

const MLP_W1: usize = 0;
const MLP_B1: usize = MLP_W1 + MLP_INPUT_SIZE * MLP_HIDDEN_SIZE;
const MLP_W2: usize = MLP_B1 + MLP_HIDDEN_SIZE;
const MLP_B2: usize = MLP_W2 + MLP_HIDDEN_SIZE * MLP_HIDDEN2_SIZE;
const MLP_W3: usize = MLP_B2 + MLP_HIDDEN2_SIZE;
const MLP_B3: usize = MLP_W3 + MLP_HIDDEN2_SIZE;
const NUM_DIFF_PARAMS: usize = MLP_B3 + 1;
const HEURISTIC_ROUND_THRESHOLD: usize = NUM_DIFF_PARAMS;
const HEURISTIC_LOOKAHEAD: usize = HEURISTIC_ROUND_THRESHOLD + 1;
const PROGRESSIVE_BIAS_WEIGHT: usize = HEURISTIC_LOOKAHEAD + 1;

// ── Node types ──

#[derive(Clone, Debug)]
enum DiffEvalNode {
    // Game state inputs (full state features)
    InputPlayerFeatures,
    InputSharedState,
    // MLP layers
    HiddenLayer1,
    HiddenLayer2,
    OutputLayer,
    WinProbability,
}

impl DiffEvalNode {
    fn title(&self) -> &'static str {
        match self {
            Self::InputPlayerFeatures => "Player Features (249x2)",
            Self::InputSharedState => "Shared State (115)",
            Self::HiddenLayer1 => "Hidden Layer 1 (613->256 LeakyReLU)",
            Self::HiddenLayer2 => "Hidden Layer 2 (256->64 LeakyReLU)",
            Self::OutputLayer => "Output Layer (64->1)",
            Self::WinProbability => "Win Probability (softmax)",
        }
    }

    fn num_inputs(&self) -> usize {
        match self {
            Self::InputPlayerFeatures | Self::InputSharedState => 0,
            Self::HiddenLayer1 => 2, // Player features + Shared state
            Self::HiddenLayer2 => 1,
            Self::OutputLayer => 1,
            Self::WinProbability => 1,
        }
    }

    fn num_outputs(&self) -> usize {
        match self {
            Self::WinProbability => 0,
            _ => 1,
        }
    }

    fn input_label(&self, idx: usize) -> &'static str {
        match self {
            Self::HiddenLayer1 => match idx {
                0 => "players [498]",
                1 => "shared [115]",
                _ => "",
            },
            Self::HiddenLayer2 => "hidden1 [256]",
            Self::OutputLayer => "hidden2 [64]",
            Self::WinProbability => "logit",
            _ => "",
        }
    }

    fn output_label(&self, _idx: usize) -> &'static str {
        "out"
    }
}

// ── Viewer implementation ──

struct DiffEvalViewer<'a> {
    params: &'a DiffEvalParams,
}

impl DiffEvalViewer<'_> {
    fn w(ui: &mut egui::Ui, label: &str, value: f64) {
        ui.colored_label(weight_color(value), format!("{:<20} {:>8.4}", label, value));
    }

    fn render_hidden1_body(&self, ui: &mut egui::Ui) {
        let w = &self.params.weights;
        ui.label(format!("W1: {}x{} = {} weights", MLP_INPUT_SIZE, MLP_HIDDEN_SIZE, MLP_INPUT_SIZE * MLP_HIDDEN_SIZE));
        ui.label(format!("B1: {} biases", MLP_HIDDEN_SIZE));
        ui.separator();

        // Show weight statistics per input group
        let groups: &[(&str, usize, usize)] = &[
            ("P0 colors [0-11]", 0, 12),
            ("P0 mats [12-14]", 12, 15),
            ("P0 ducats [15]", 15, 16),
            ("P0 sell ct [16]", 16, 17),
            ("P0 sell duc [17]", 17, 18),
            ("P0 acted [18]", 18, 19),
            ("P0 deck [19-64]", 19, 65),
            ("P0 discard [65-110]", 65, 111),
            ("P0 drafted [111-156]", 111, 157),
            ("P0 workshop [157-202]", 157, 203),
            ("P0 workshopped [203-248]", 203, 249),
            ("P1 (same layout) [249-497]", 249, 498),
            ("Shared state [498-612]", 498, 613),
        ];

        ui.label("Mean |W1| per input group:");
        for &(name, start, end) in groups {
            let mut sum = 0.0f64;
            let count = (end - start) * MLP_HIDDEN_SIZE;
            for col in start..end {
                for row in 0..MLP_HIDDEN_SIZE {
                    sum += w[MLP_W1 + row * MLP_INPUT_SIZE + col].abs();
                }
            }
            let mean = if count > 0 { sum / count as f64 } else { 0.0 };
            Self::w(ui, name, mean);
        }
    }

    fn render_hidden2_body(&self, ui: &mut egui::Ui) {
        let w = &self.params.weights;
        ui.label(format!("W2: {}x{} = {} weights", MLP_HIDDEN_SIZE, MLP_HIDDEN2_SIZE, MLP_HIDDEN_SIZE * MLP_HIDDEN2_SIZE));
        ui.label(format!("B2: {} biases", MLP_HIDDEN2_SIZE));
        ui.separator();

        let mut sum_abs = 0.0f64;
        let mut max_abs = 0.0f64;
        for i in 0..(MLP_HIDDEN_SIZE * MLP_HIDDEN2_SIZE) {
            let abs = w[MLP_W2 + i].abs();
            sum_abs += abs;
            if abs > max_abs { max_abs = abs; }
        }
        Self::w(ui, "mean |W2|", sum_abs / (MLP_HIDDEN_SIZE * MLP_HIDDEN2_SIZE) as f64);
        Self::w(ui, "max |W2|", max_abs);
    }

    fn render_output_layer_body(&self, ui: &mut egui::Ui) {
        let w = &self.params.weights;
        ui.label(format!("W3: {} weights", MLP_HIDDEN2_SIZE));
        ui.separator();

        let mut sum_abs = 0.0f64;
        let mut max_abs = 0.0f64;
        for i in 0..MLP_HIDDEN2_SIZE {
            let abs = w[MLP_W3 + i].abs();
            sum_abs += abs;
            if abs > max_abs { max_abs = abs; }
        }
        Self::w(ui, "mean |W3|", sum_abs / MLP_HIDDEN2_SIZE as f64);
        Self::w(ui, "max |W3|", max_abs);
        Self::w(ui, "bias", w[MLP_B3]);
    }

    fn render_control_params(&self, ui: &mut egui::Ui) {
        let w = &self.params.weights;
        Self::w(ui, "round_threshold", w[HEURISTIC_ROUND_THRESHOLD]);
        Self::w(ui, "lookahead", w[HEURISTIC_LOOKAHEAD]);
        Self::w(ui, "progressive_bias", w[PROGRESSIVE_BIAS_WEIGHT]);
    }
}

fn weight_color(value: f64) -> egui::Color32 {
    let abs = value.abs().min(2.0);
    let intensity = (abs / 2.0 * 200.0) as u8;
    if value > 0.001 {
        egui::Color32::from_rgb(100 + intensity / 2, 200, 100 + intensity / 2)
    } else if value < -0.001 {
        egui::Color32::from_rgb(200, 100 + intensity / 2, 100 + intensity / 2)
    } else {
        egui::Color32::GRAY
    }
}

impl SnarlViewer<DiffEvalNode> for DiffEvalViewer<'_> {
    fn title(&mut self, node: &DiffEvalNode) -> String {
        node.title().to_string()
    }

    fn inputs(&mut self, node: &DiffEvalNode) -> usize {
        node.num_inputs()
    }

    fn outputs(&mut self, node: &DiffEvalNode) -> usize {
        node.num_outputs()
    }

    #[allow(refining_impl_trait)]
    fn show_input(&mut self, pin: &InPin, ui: &mut egui::Ui, snarl: &mut Snarl<DiffEvalNode>) -> PinInfo {
        let node = &snarl[pin.id.node];
        ui.label(node.input_label(pin.id.input));
        PinInfo::circle().with_fill(egui::Color32::from_rgb(150, 150, 200))
    }

    #[allow(refining_impl_trait)]
    fn show_output(&mut self, pin: &OutPin, ui: &mut egui::Ui, snarl: &mut Snarl<DiffEvalNode>) -> PinInfo {
        let node = &snarl[pin.id.node];
        ui.label(node.output_label(pin.id.output));
        PinInfo::circle().with_fill(egui::Color32::from_rgb(200, 150, 150))
    }

    fn has_on_hover_popup(&mut self, node: &DiffEvalNode) -> bool {
        !matches!(node,
            DiffEvalNode::InputPlayerFeatures | DiffEvalNode::InputSharedState
        )
    }

    fn show_on_hover_popup(
        &mut self,
        node: NodeId,
        _inputs: &[InPin],
        _outputs: &[OutPin],
        ui: &mut egui::Ui,
        snarl: &mut Snarl<DiffEvalNode>,
    ) {
        let node_data = &snarl[node].clone();
        match node_data {
            DiffEvalNode::HiddenLayer1 => self.render_hidden1_body(ui),
            DiffEvalNode::HiddenLayer2 => self.render_hidden2_body(ui),
            DiffEvalNode::OutputLayer => self.render_output_layer_body(ui),
            DiffEvalNode::WinProbability => self.render_control_params(ui),
            _ => {}
        }
    }

    fn connect(&mut self, _from: &OutPin, _to: &InPin, _snarl: &mut Snarl<DiffEvalNode>) {}
    fn disconnect(&mut self, _from: &OutPin, _to: &InPin, _snarl: &mut Snarl<DiffEvalNode>) {}
}

// ── Graph construction ──

fn build_graph() -> Snarl<DiffEvalNode> {
    let mut snarl = Snarl::new();

    let col_input = 0.0;
    let col_mlp = 400.0;
    let col_output = 800.0;
    let spacing = 70.0;

    // Input nodes
    let player_id = snarl.insert_node(egui::pos2(col_input, 1.0 * spacing), DiffEvalNode::InputPlayerFeatures);
    let shared_id = snarl.insert_node(egui::pos2(col_input, 3.0 * spacing), DiffEvalNode::InputSharedState);

    // MLP nodes
    let hidden1_id = snarl.insert_node(egui::pos2(col_mlp, 1.0 * spacing), DiffEvalNode::HiddenLayer1);
    let hidden2_id = snarl.insert_node(egui::pos2(col_mlp, 3.0 * spacing), DiffEvalNode::HiddenLayer2);
    let output_id = snarl.insert_node(egui::pos2(col_mlp, 5.0 * spacing), DiffEvalNode::OutputLayer);

    // Output node
    let win_id = snarl.insert_node(egui::pos2(col_output, 3.0 * spacing), DiffEvalNode::WinProbability);

    // Input -> Hidden Layer 1
    snarl.connect(OutPinId { node: player_id, output: 0 }, InPinId { node: hidden1_id, input: 0 });
    snarl.connect(OutPinId { node: shared_id, output: 0 }, InPinId { node: hidden1_id, input: 1 });

    // Hidden1 -> Hidden2 -> Output -> Win Probability
    snarl.connect(OutPinId { node: hidden1_id, output: 0 }, InPinId { node: hidden2_id, input: 0 });
    snarl.connect(OutPinId { node: hidden2_id, output: 0 }, InPinId { node: output_id, input: 0 });
    snarl.connect(OutPinId { node: output_id, output: 0 }, InPinId { node: win_id, input: 0 });

    snarl
}

// ── Tab state ──

pub struct DiffEvalViewerState {
    params: Option<DiffEvalParams>,
    snarl: Snarl<DiffEvalNode>,
    style: SnarlStyle,
    loaded_path: Option<String>,
    error: Option<String>,
}

impl DiffEvalViewerState {
    pub fn new() -> Self {
        Self {
            params: None,
            snarl: build_graph(),
            style: SnarlStyle::new(),
            loaded_path: None,
            error: None,
        }
    }

    pub fn try_auto_load(&mut self) {
        if let Ok(cwd) = std::env::current_dir() {
            let path = cwd.join("diff-eval-training/latest-diff-eval.json");
            if path.is_file() {
                self.load_file(&path);
            }
        }
    }

    fn load_file(&mut self, path: &std::path::Path) {
        match std::fs::read_to_string(path) {
            Ok(contents) => {
                match serde_json::from_str::<DiffEvalParams>(&contents) {
                    Ok(params) => {
                        self.params = Some(params);
                        self.loaded_path = Some(path.display().to_string());
                        self.error = None;
                    }
                    Err(e) => {
                        self.error = Some(format!("Failed to parse: {}", e));
                    }
                }
            }
            Err(e) => {
                self.error = Some(format!("Failed to read: {}", e));
            }
        }
    }

    pub fn render(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("Load Params").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("JSON", &["json"])
                    .pick_file()
                {
                    self.load_file(&path);
                }
            }
            if let Some(ref path) = self.loaded_path {
                ui.label(format!("Loaded: {}", path));
            }
            if let Some(ref err) = self.error {
                ui.colored_label(egui::Color32::RED, err);
            }
        });
        ui.separator();

        if let Some(ref params) = self.params {
            let params = params.clone();
            let mut viewer = DiffEvalViewer { params: &params };
            self.snarl.show(&mut viewer, &self.style, egui::Id::new("diff_eval_snarl"), ui);
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("Load a diff eval params file to view the computation graph");
            });
        }
    }
}
