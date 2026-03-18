use eframe::egui;
use egui_snarl::ui::{PinInfo, SnarlViewer, SnarlStyle};
use egui_snarl::{InPin, InPinId, NodeId, OutPin, OutPinId, Snarl};

use colori_core::scoring::diff_eval::{DiffEvalParams, MLP_INPUT_SIZE, MLP_HIDDEN_SIZE};

// ── Parameter index constants (mirrored from diff_eval.rs) ──

const COLOR_SAT_W: usize = 0;
const COLOR_SAT_A: usize = 3;
const MIX_PAIR_W: usize = 6;
const COVERAGE_W: usize = 15;
const COVERAGE_A: usize = 18;
const COVERAGE_B: usize = 21;

const SELL_MAT_W: usize = 24;
const SELL_DUCAT_W: usize = 27;
const SELL_COMBINE_W: usize = 30;
const SELL_AGG_W: usize = 32;
const SELL_ROUND_W: usize = 35;
const SELL_SOLD_W: usize = 37;

const DECK_COLOR_SAT_W: usize = 40;
const DECK_COLOR_SAT_A: usize = 43;
const DECK_PROD_NEED_W: usize = 46;
const DECK_ACTION_W: usize = 49;
const DECK_MAT_CARD_W: usize = 54;
const DECK_SIZE_W: usize = 57;
const DECK_DIVERSITY_W: usize = 59;
const DECK_WORKSHOP_W: usize = 60;

const MAT_SUFF_W: usize = 61;
const MAT_SUFF_THRESH: usize = 64;
const MAT_DEMAND_W: usize = 67;
const MAT_DIVERSITY_W: usize = 70;

const MLP_W1: usize = 72;
const MLP_B1: usize = MLP_W1 + MLP_INPUT_SIZE * MLP_HIDDEN_SIZE;
const MLP_W2: usize = MLP_B1 + MLP_HIDDEN_SIZE;
const MLP_B2: usize = MLP_W2 + MLP_HIDDEN_SIZE;
const HEURISTIC_ROUND_THRESHOLD: usize = MLP_B2 + 1;
const HEURISTIC_LOOKAHEAD: usize = HEURISTIC_ROUND_THRESHOLD + 1;
const PROGRESSIVE_BIAS_WEIGHT: usize = HEURISTIC_LOOKAHEAD + 1;

// ── Node types ──

#[derive(Clone, Debug)]
enum DiffEvalNode {
    // Game state inputs (read by modules)
    InputScore,
    InputColorWheel,
    InputMaterials,
    InputSellCards,
    InputDeck,
    InputRound,
    // Modules (hand-crafted features with learnable params)
    ColorWheelValue,
    SellCardAlignment,
    DeckColorProfile,
    MaterialStrategy,
    // Raw features (passed directly to MLP)
    RawFeatures,
    // MLP
    HiddenLayer,
    OutputLayer,
    WinProbability,
}

impl DiffEvalNode {
    fn title(&self) -> &'static str {
        match self {
            Self::InputScore => "Score / 20",
            Self::InputColorWheel => "Color Wheel",
            Self::InputMaterials => "Materials",
            Self::InputSellCards => "Sell Card Display",
            Self::InputDeck => "Deck",
            Self::InputRound => "Round / 20",
            Self::ColorWheelValue => "Color Wheel Value (24 params)",
            Self::SellCardAlignment => "Sell Card Alignment (17 params)",
            Self::DeckColorProfile => "Deck Color Profile (22 params)",
            Self::MaterialStrategy => "Material Strategy (11 params)",
            Self::RawFeatures => "Raw Features (87 values)",
            Self::HiddenLayer => "Hidden Layer (117->256 ReLU)",
            Self::OutputLayer => "Output Layer (256->1)",
            Self::WinProbability => "Win Probability (softmax)",
        }
    }

    fn num_inputs(&self) -> usize {
        match self {
            Self::InputScore | Self::InputColorWheel | Self::InputMaterials
            | Self::InputSellCards | Self::InputDeck | Self::InputRound => 0,
            Self::ColorWheelValue => 1,
            Self::SellCardAlignment => 4,
            Self::DeckColorProfile => 2,
            Self::MaterialStrategy => 2,
            Self::RawFeatures => 4, // Color Wheel, Deck, Materials, Sell Cards
            Self::HiddenLayer => 7, // CWV(7) + SCA(5) + DCP(9) + MS(7) + direct(2) + raw(87) — grouped as 7 source connections
            Self::OutputLayer => 1,
            Self::WinProbability => 1,
        }
    }

    fn num_outputs(&self) -> usize {
        match self {
            Self::WinProbability => 0,
            Self::ColorWheelValue => 7,
            Self::SellCardAlignment => 5,
            Self::DeckColorProfile => 9,
            Self::MaterialStrategy => 7,
            Self::RawFeatures => 1,
            _ => 1,
        }
    }

    fn input_label(&self, idx: usize) -> &'static str {
        match self {
            Self::ColorWheelValue => "Color Wheel",
            Self::SellCardAlignment => match idx {
                0 => "Color Wheel",
                1 => "Materials",
                2 => "Sell Cards",
                3 => "Round",
                _ => "",
            },
            Self::DeckColorProfile => match idx {
                0 => "Deck",
                1 => "Sell Cards",
                _ => "",
            },
            Self::MaterialStrategy => match idx {
                0 => "Materials",
                1 => "Sell Cards",
                _ => "",
            },
            Self::RawFeatures => match idx {
                0 => "Color Wheel",
                1 => "Deck",
                2 => "Materials",
                3 => "Sell Cards",
                _ => "",
            },
            Self::HiddenLayer => match idx {
                0 => "CWV [7]",
                1 => "SCA [5]",
                2 => "DCP [9]",
                3 => "MS [7]",
                4 => "score/20",
                5 => "round/20",
                6 => "raw [87]",
                _ => "",
            },
            Self::OutputLayer => "hidden [256]",
            Self::WinProbability => "logit",
            _ => "",
        }
    }

    fn output_label(&self, idx: usize) -> &'static str {
        match self {
            Self::ColorWheelValue => match idx {
                0 => "pri sat",
                1 => "sec sat",
                2 => "ter sat",
                3 => "mix pairs",
                4 => "pri cov",
                5 => "sec cov",
                6 => "ter cov",
                _ => "out",
            },
            Self::SellCardAlignment => match idx {
                0 => "best",
                1 => "second",
                2 => "rest",
                3 => "urgency",
                4 => "sold",
                _ => "out",
            },
            Self::DeckColorProfile => match idx {
                0 => "pri prod",
                1 => "sec prod",
                2 => "ter prod",
                3 => "need",
                4 => "action",
                5 => "mat cards",
                6 => "size",
                7 => "diversity",
                8 => "workshop",
                _ => "out",
            },
            Self::MaterialStrategy => match idx {
                0 => "tex suff",
                1 => "cer suff",
                2 => "pnt suff",
                3 => "tex demand",
                4 => "cer demand",
                5 => "pnt demand",
                6 => "diversity",
                _ => "out",
            },
            _ => "out",
        }
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

    fn render_color_wheel_body(&self, ui: &mut egui::Ui) {
        let w = &self.params.weights;
        ui.label("Outputs: 7 (3 tier sat + 1 mix + 3 coverage)");
        ui.separator();
        for (i, tier) in ["Primary", "Secondary", "Tertiary"].iter().enumerate() {
            Self::w(ui, &format!("{} sat_w", tier), w[COLOR_SAT_W + i]);
            Self::w(ui, &format!("{} sat_a", tier), w[COLOR_SAT_A + i]);
        }
        ui.separator();
        let pair_names = [
            "Red+Yellow", "Red+Blue", "Yellow+Blue",
            "Red+Orange", "Yellow+Orange", "Yellow+Green",
            "Blue+Green", "Blue+Purple", "Red+Purple",
        ];
        for (i, name) in pair_names.iter().enumerate() {
            Self::w(ui, &format!("Mix {}", name), w[MIX_PAIR_W + i]);
        }
        ui.separator();
        for (i, tier) in ["Primary", "Secondary", "Tertiary"].iter().enumerate() {
            Self::w(ui, &format!("{} cov_w", tier), w[COVERAGE_W + i]);
            Self::w(ui, &format!("{} cov_a", tier), w[COVERAGE_A + i]);
            Self::w(ui, &format!("{} cov_b", tier), w[COVERAGE_B + i]);
        }
    }

    fn render_sell_card_body(&self, ui: &mut egui::Ui) {
        let w = &self.params.weights;
        ui.label("Outputs: 5 (best, second, rest, urgency, sold)");
        ui.separator();
        for (i, name) in ["Textiles", "Ceramics", "Paintings"].iter().enumerate() {
            Self::w(ui, &format!("{} mat_w", name), w[SELL_MAT_W + i]);
        }
        ui.separator();
        for (i, name) in ["2-ducat", "3-ducat", "4-ducat"].iter().enumerate() {
            Self::w(ui, &format!("{} color_w", name), w[SELL_DUCAT_W + i]);
        }
        ui.separator();
        Self::w(ui, "combine_mat", w[SELL_COMBINE_W]);
        Self::w(ui, "combine_color", w[SELL_COMBINE_W + 1]);
        ui.separator();
        Self::w(ui, "agg_best", w[SELL_AGG_W]);
        Self::w(ui, "agg_second", w[SELL_AGG_W + 1]);
        Self::w(ui, "agg_rest", w[SELL_AGG_W + 2]);
        ui.separator();
        Self::w(ui, "round_w", w[SELL_ROUND_W]);
        Self::w(ui, "round_b", w[SELL_ROUND_W + 1]);
        ui.separator();
        Self::w(ui, "sold_w", w[SELL_SOLD_W]);
        Self::w(ui, "sold_a", w[SELL_SOLD_W + 1]);
        Self::w(ui, "sold_b", w[SELL_SOLD_W + 2]);
    }

    fn render_deck_profile_body(&self, ui: &mut egui::Ui) {
        let w = &self.params.weights;
        ui.label("Outputs: 9 (3 tier prod + need + action + mat + size + div + workshop)");
        ui.separator();
        for (i, tier) in ["Primary", "Secondary", "Tertiary"].iter().enumerate() {
            Self::w(ui, &format!("{} sat_w", tier), w[DECK_COLOR_SAT_W + i]);
            Self::w(ui, &format!("{} sat_a", tier), w[DECK_COLOR_SAT_A + i]);
        }
        ui.separator();
        for (i, name) in ["2-ducat", "3-ducat", "4-ducat"].iter().enumerate() {
            Self::w(ui, &format!("{} need_w", name), w[DECK_PROD_NEED_W + i]);
        }
        ui.separator();
        for (i, name) in ["Alum", "CreamOfTartar", "GumArabic", "Potash", "Chalk"].iter().enumerate() {
            Self::w(ui, name, w[DECK_ACTION_W + i]);
        }
        ui.separator();
        for (i, name) in ["Starter mat", "Colored mat", "Dual mat"].iter().enumerate() {
            Self::w(ui, name, w[DECK_MAT_CARD_W + i]);
        }
        ui.separator();
        Self::w(ui, "size_linear", w[DECK_SIZE_W]);
        Self::w(ui, "size_quad", w[DECK_SIZE_W + 1]);
        Self::w(ui, "diversity", w[DECK_DIVERSITY_W]);
        Self::w(ui, "workshopped", w[DECK_WORKSHOP_W]);
    }

    fn render_material_body(&self, ui: &mut egui::Ui) {
        let w = &self.params.weights;
        ui.label("Outputs: 7 (3 sufficiency + 3 demand + 1 diversity)");
        ui.separator();
        for (i, name) in ["Textiles", "Ceramics", "Paintings"].iter().enumerate() {
            Self::w(ui, &format!("{} suff_w", name), w[MAT_SUFF_W + i]);
            Self::w(ui, &format!("{} thresh", name), w[MAT_SUFF_THRESH + i]);
        }
        ui.separator();
        for (i, name) in ["Textiles", "Ceramics", "Paintings"].iter().enumerate() {
            Self::w(ui, &format!("{} demand", name), w[MAT_DEMAND_W + i]);
        }
        ui.separator();
        Self::w(ui, "diversity_2+", w[MAT_DIVERSITY_W]);
        Self::w(ui, "diversity_3", w[MAT_DIVERSITY_W + 1]);
    }

    fn render_hidden_layer_body(&self, ui: &mut egui::Ui) {
        let w = &self.params.weights;
        ui.label(format!("W1: {}x{} = {} weights", MLP_INPUT_SIZE, MLP_HIDDEN_SIZE, MLP_INPUT_SIZE * MLP_HIDDEN_SIZE));
        ui.label(format!("B1: {} biases", MLP_HIDDEN_SIZE));
        ui.separator();

        // Show weight statistics per input group
        let groups: &[(&str, usize, usize)] = &[
            ("CWV [0-6]", 0, 7),
            ("SCA [7-11]", 7, 12),
            ("DCP [12-20]", 12, 21),
            ("MS [21-27]", 21, 28),
            ("score/20", 28, 29),
            ("round/20", 29, 30),
            ("raw colors [30-41]", 30, 42),
            ("raw prod [42-53]", 42, 54),
            ("raw demand [54-65]", 54, 66),
            ("raw cards [66-111]", 66, 112),
            ("raw mats [112-114]", 112, 115),
            ("raw other [115-116]", 115, 117),
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
            let mean = sum / count as f64;
            Self::w(ui, name, mean);
        }
    }

    fn render_output_layer_body(&self, ui: &mut egui::Ui) {
        let w = &self.params.weights;
        ui.label(format!("W2: {} weights", MLP_HIDDEN_SIZE));
        ui.separator();

        // Show weight statistics
        let mut sum_abs = 0.0f64;
        let mut max_abs = 0.0f64;
        for i in 0..MLP_HIDDEN_SIZE {
            let abs = w[MLP_W2 + i].abs();
            sum_abs += abs;
            if abs > max_abs { max_abs = abs; }
        }
        Self::w(ui, "mean |W2|", sum_abs / MLP_HIDDEN_SIZE as f64);
        Self::w(ui, "max |W2|", max_abs);
        Self::w(ui, "bias", w[MLP_B2]);
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
            DiffEvalNode::InputScore | DiffEvalNode::InputColorWheel |
            DiffEvalNode::InputMaterials | DiffEvalNode::InputSellCards |
            DiffEvalNode::InputDeck | DiffEvalNode::InputRound |
            DiffEvalNode::RawFeatures
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
            DiffEvalNode::ColorWheelValue => self.render_color_wheel_body(ui),
            DiffEvalNode::SellCardAlignment => self.render_sell_card_body(ui),
            DiffEvalNode::DeckColorProfile => self.render_deck_profile_body(ui),
            DiffEvalNode::MaterialStrategy => self.render_material_body(ui),
            DiffEvalNode::HiddenLayer => self.render_hidden_layer_body(ui),
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
    let col_module = 300.0;
    let col_mlp = 700.0;
    let col_output = 1050.0;
    let spacing = 70.0;

    // Input nodes
    let score_id = snarl.insert_node(egui::pos2(col_input, 0.0 * spacing), DiffEvalNode::InputScore);
    let color_id = snarl.insert_node(egui::pos2(col_input, 1.0 * spacing), DiffEvalNode::InputColorWheel);
    let sell_id = snarl.insert_node(egui::pos2(col_input, 2.5 * spacing), DiffEvalNode::InputSellCards);
    let deck_id = snarl.insert_node(egui::pos2(col_input, 3.5 * spacing), DiffEvalNode::InputDeck);
    let mat_id = snarl.insert_node(egui::pos2(col_input, 4.5 * spacing), DiffEvalNode::InputMaterials);
    let round_id = snarl.insert_node(egui::pos2(col_input, 5.5 * spacing), DiffEvalNode::InputRound);

    // Module nodes
    let cwv_id = snarl.insert_node(egui::pos2(col_module, 0.0 * spacing), DiffEvalNode::ColorWheelValue);
    let sca_id = snarl.insert_node(egui::pos2(col_module, 2.0 * spacing), DiffEvalNode::SellCardAlignment);
    let dcp_id = snarl.insert_node(egui::pos2(col_module, 4.0 * spacing), DiffEvalNode::DeckColorProfile);
    let ms_id = snarl.insert_node(egui::pos2(col_module, 6.5 * spacing), DiffEvalNode::MaterialStrategy);
    let raw_id = snarl.insert_node(egui::pos2(col_module, 8.5 * spacing), DiffEvalNode::RawFeatures);

    // MLP nodes
    let hidden_id = snarl.insert_node(egui::pos2(col_mlp, 3.0 * spacing), DiffEvalNode::HiddenLayer);
    let output_id = snarl.insert_node(egui::pos2(col_mlp, 6.0 * spacing), DiffEvalNode::OutputLayer);

    // Output node
    let win_id = snarl.insert_node(egui::pos2(col_output, 4.5 * spacing), DiffEvalNode::WinProbability);

    // Input -> Module edges
    snarl.connect(OutPinId { node: color_id, output: 0 }, InPinId { node: cwv_id, input: 0 });
    snarl.connect(OutPinId { node: color_id, output: 0 }, InPinId { node: sca_id, input: 0 });
    snarl.connect(OutPinId { node: mat_id, output: 0 }, InPinId { node: sca_id, input: 1 });
    snarl.connect(OutPinId { node: sell_id, output: 0 }, InPinId { node: sca_id, input: 2 });
    snarl.connect(OutPinId { node: round_id, output: 0 }, InPinId { node: sca_id, input: 3 });
    snarl.connect(OutPinId { node: deck_id, output: 0 }, InPinId { node: dcp_id, input: 0 });
    snarl.connect(OutPinId { node: sell_id, output: 0 }, InPinId { node: dcp_id, input: 1 });
    snarl.connect(OutPinId { node: mat_id, output: 0 }, InPinId { node: ms_id, input: 0 });
    snarl.connect(OutPinId { node: sell_id, output: 0 }, InPinId { node: ms_id, input: 1 });

    // Input -> Raw features
    snarl.connect(OutPinId { node: color_id, output: 0 }, InPinId { node: raw_id, input: 0 });
    snarl.connect(OutPinId { node: deck_id, output: 0 }, InPinId { node: raw_id, input: 1 });
    snarl.connect(OutPinId { node: mat_id, output: 0 }, InPinId { node: raw_id, input: 2 });
    snarl.connect(OutPinId { node: sell_id, output: 0 }, InPinId { node: raw_id, input: 3 });

    // Module -> Hidden Layer (using output pin 0 for each — the connections are conceptual)
    snarl.connect(OutPinId { node: cwv_id, output: 0 }, InPinId { node: hidden_id, input: 0 });
    snarl.connect(OutPinId { node: sca_id, output: 0 }, InPinId { node: hidden_id, input: 1 });
    snarl.connect(OutPinId { node: dcp_id, output: 0 }, InPinId { node: hidden_id, input: 2 });
    snarl.connect(OutPinId { node: ms_id, output: 0 }, InPinId { node: hidden_id, input: 3 });
    snarl.connect(OutPinId { node: score_id, output: 0 }, InPinId { node: hidden_id, input: 4 });
    snarl.connect(OutPinId { node: round_id, output: 0 }, InPinId { node: hidden_id, input: 5 });
    snarl.connect(OutPinId { node: raw_id, output: 0 }, InPinId { node: hidden_id, input: 6 });

    // Hidden -> Output -> Win Probability
    snarl.connect(OutPinId { node: hidden_id, output: 0 }, InPinId { node: output_id, input: 0 });
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
