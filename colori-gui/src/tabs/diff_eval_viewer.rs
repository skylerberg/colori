use eframe::egui;
use egui_snarl::ui::{PinInfo, SnarlViewer, SnarlStyle};
use egui_snarl::{InPin, InPinId, NodeId, OutPin, OutPinId, Snarl};

use colori_core::scoring::diff_eval::DiffEvalParams;

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
const MLP_B1: usize = 168;
const MLP_W2: usize = 184;
const MLP_B2: usize = 200;
const HEURISTIC_ROUND_THRESHOLD: usize = 201;
const HEURISTIC_LOOKAHEAD: usize = 202;
const PROGRESSIVE_BIAS_WEIGHT: usize = 203;

// ── Node types ──

#[derive(Clone, Debug)]
enum DiffEvalNode {
    InputScore,
    InputColorWheel,
    InputMaterials,
    InputSellCards,
    InputDeck,
    InputRound,
    ColorWheelValue,
    SellCardAlignment,
    DeckColorProfile,
    MaterialStrategy,
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
            Self::ColorWheelValue => "Color Wheel Value",
            Self::SellCardAlignment => "Sell Card Alignment",
            Self::DeckColorProfile => "Deck Color Profile",
            Self::MaterialStrategy => "Material Strategy",
            Self::HiddenLayer => "Hidden Layer (6->16 ReLU)",
            Self::OutputLayer => "Output Layer (16->1)",
            Self::WinProbability => "Win Probability",
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
            Self::HiddenLayer => 6,
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
            Self::HiddenLayer => match idx {
                0 => "score/20",
                1 => "color_value",
                2 => "sell_align",
                3 => "deck_profile",
                4 => "material",
                5 => "round/20",
                _ => "",
            },
            Self::OutputLayer => "hidden[16]",
            Self::WinProbability => "logit",
            _ => "",
        }
    }
}

// ── Viewer implementation ──

struct DiffEvalViewer<'a> {
    params: &'a DiffEvalParams,
}

impl DiffEvalViewer<'_> {
    fn render_weight_row(ui: &mut egui::Ui, label: &str, value: f64) {
        ui.label(label);
        let color = weight_color(value);
        ui.colored_label(color, format!("{:.4}", value));
        ui.end_row();
    }

    fn render_color_wheel_body(&self, ui: &mut egui::Ui) {
        let w = &self.params.weights;
        egui::Grid::new("cwv_params").num_columns(2).show(ui, |ui| {
            for (i, tier) in ["Primary", "Secondary", "Tertiary"].iter().enumerate() {
                Self::render_weight_row(ui, &format!("{} sat_w", tier), w[COLOR_SAT_W + i]);
                Self::render_weight_row(ui, &format!("{} sat_a", tier), w[COLOR_SAT_A + i]);
            }
            ui.separator(); ui.separator(); ui.end_row();
            let pair_names = [
                "Red+Yellow", "Red+Blue", "Yellow+Blue",
                "Red+Orange", "Yellow+Orange", "Yellow+Green",
                "Blue+Green", "Blue+Purple", "Red+Purple",
            ];
            for (i, name) in pair_names.iter().enumerate() {
                Self::render_weight_row(ui, &format!("Mix {}", name), w[MIX_PAIR_W + i]);
            }
            ui.separator(); ui.separator(); ui.end_row();
            for (i, tier) in ["Primary", "Secondary", "Tertiary"].iter().enumerate() {
                Self::render_weight_row(ui, &format!("{} cov_w", tier), w[COVERAGE_W + i]);
                Self::render_weight_row(ui, &format!("{} cov_a", tier), w[COVERAGE_A + i]);
                Self::render_weight_row(ui, &format!("{} cov_b", tier), w[COVERAGE_B + i]);
            }
        });
    }

    fn render_sell_card_body(&self, ui: &mut egui::Ui) {
        let w = &self.params.weights;
        egui::Grid::new("sca_params").num_columns(2).show(ui, |ui| {
            for (i, name) in ["Textiles", "Ceramics", "Paintings"].iter().enumerate() {
                Self::render_weight_row(ui, &format!("{} mat_w", name), w[SELL_MAT_W + i]);
            }
            ui.separator(); ui.separator(); ui.end_row();
            for (i, name) in ["2-ducat", "3-ducat", "4-ducat"].iter().enumerate() {
                Self::render_weight_row(ui, &format!("{} color_w", name), w[SELL_DUCAT_W + i]);
            }
            ui.separator(); ui.separator(); ui.end_row();
            Self::render_weight_row(ui, "combine_mat", w[SELL_COMBINE_W]);
            Self::render_weight_row(ui, "combine_color", w[SELL_COMBINE_W + 1]);
            ui.separator(); ui.separator(); ui.end_row();
            Self::render_weight_row(ui, "agg_best", w[SELL_AGG_W]);
            Self::render_weight_row(ui, "agg_second", w[SELL_AGG_W + 1]);
            Self::render_weight_row(ui, "agg_rest", w[SELL_AGG_W + 2]);
            ui.separator(); ui.separator(); ui.end_row();
            Self::render_weight_row(ui, "round_w", w[SELL_ROUND_W]);
            Self::render_weight_row(ui, "round_b", w[SELL_ROUND_W + 1]);
            ui.separator(); ui.separator(); ui.end_row();
            Self::render_weight_row(ui, "sold_w", w[SELL_SOLD_W]);
            Self::render_weight_row(ui, "sold_a", w[SELL_SOLD_W + 1]);
            Self::render_weight_row(ui, "sold_b", w[SELL_SOLD_W + 2]);
        });
    }

    fn render_deck_profile_body(&self, ui: &mut egui::Ui) {
        let w = &self.params.weights;
        egui::Grid::new("dcp_params").num_columns(2).show(ui, |ui| {
            for (i, tier) in ["Primary", "Secondary", "Tertiary"].iter().enumerate() {
                Self::render_weight_row(ui, &format!("{} sat_w", tier), w[DECK_COLOR_SAT_W + i]);
                Self::render_weight_row(ui, &format!("{} sat_a", tier), w[DECK_COLOR_SAT_A + i]);
            }
            ui.separator(); ui.separator(); ui.end_row();
            for (i, name) in ["2-ducat", "3-ducat", "4-ducat"].iter().enumerate() {
                Self::render_weight_row(ui, &format!("{} need_w", name), w[DECK_PROD_NEED_W + i]);
            }
            ui.separator(); ui.separator(); ui.end_row();
            for (i, name) in ["Alum", "CreamOfTartar", "GumArabic", "Potash", "Chalk"].iter().enumerate() {
                Self::render_weight_row(ui, name, w[DECK_ACTION_W + i]);
            }
            ui.separator(); ui.separator(); ui.end_row();
            for (i, name) in ["Starter mat", "Colored mat", "Dual mat"].iter().enumerate() {
                Self::render_weight_row(ui, name, w[DECK_MAT_CARD_W + i]);
            }
            ui.separator(); ui.separator(); ui.end_row();
            Self::render_weight_row(ui, "size_linear", w[DECK_SIZE_W]);
            Self::render_weight_row(ui, "size_quad", w[DECK_SIZE_W + 1]);
            Self::render_weight_row(ui, "diversity", w[DECK_DIVERSITY_W]);
            Self::render_weight_row(ui, "workshopped", w[DECK_WORKSHOP_W]);
        });
    }

    fn render_material_body(&self, ui: &mut egui::Ui) {
        let w = &self.params.weights;
        egui::Grid::new("ms_params").num_columns(2).show(ui, |ui| {
            for (i, name) in ["Textiles", "Ceramics", "Paintings"].iter().enumerate() {
                Self::render_weight_row(ui, &format!("{} suff_w", name), w[MAT_SUFF_W + i]);
                Self::render_weight_row(ui, &format!("{} thresh", name), w[MAT_SUFF_THRESH + i]);
            }
            ui.separator(); ui.separator(); ui.end_row();
            for (i, name) in ["Textiles", "Ceramics", "Paintings"].iter().enumerate() {
                Self::render_weight_row(ui, &format!("{} demand", name), w[MAT_DEMAND_W + i]);
            }
            ui.separator(); ui.separator(); ui.end_row();
            Self::render_weight_row(ui, "diversity_2+", w[MAT_DIVERSITY_W]);
            Self::render_weight_row(ui, "diversity_3", w[MAT_DIVERSITY_W + 1]);
        });
    }

    fn render_hidden_layer_body(&self, ui: &mut egui::Ui) {
        let w = &self.params.weights;
        let input_names = ["score", "color", "sell", "deck", "mat", "round"];

        ui.label("W1 weights:");
        egui::Grid::new("mlp_w1").num_columns(7).spacing([4.0, 2.0]).show(ui, |ui| {
            ui.label("");
            for name in &input_names {
                ui.label(*name);
            }
            ui.end_row();
            for row in 0..16 {
                ui.label(format!("h{}", row));
                for col in 0..6 {
                    let val = w[MLP_W1 + row * 6 + col];
                    let color = weight_color(val);
                    ui.colored_label(color, format!("{:.2}", val));
                }
                ui.end_row();
            }
        });
        ui.add_space(4.0);
        ui.label("Biases:");
        egui::Grid::new("mlp_b1").num_columns(8).spacing([4.0, 2.0]).show(ui, |ui| {
            for i in 0..16 {
                let color = weight_color(w[MLP_B1 + i]);
                ui.colored_label(color, format!("{:.3}", w[MLP_B1 + i]));
                if i == 7 { ui.end_row(); }
            }
            ui.end_row();
        });
    }

    fn render_output_layer_body(&self, ui: &mut egui::Ui) {
        let w = &self.params.weights;
        ui.label("W2 weights:");
        egui::Grid::new("mlp_w2").num_columns(8).spacing([4.0, 2.0]).show(ui, |ui| {
            for i in 0..16 {
                let color = weight_color(w[MLP_W2 + i]);
                ui.colored_label(color, format!("{:.3}", w[MLP_W2 + i]));
                if i == 7 { ui.end_row(); }
            }
            ui.end_row();
        });
        ui.add_space(4.0);
        egui::Grid::new("mlp_b2").num_columns(2).show(ui, |ui| {
            Self::render_weight_row(ui, "bias", w[MLP_B2]);
        });
    }

    fn render_control_params(&self, ui: &mut egui::Ui) {
        let w = &self.params.weights;
        egui::Grid::new("ctrl_params").num_columns(2).show(ui, |ui| {
            Self::render_weight_row(ui, "round_threshold", w[HEURISTIC_ROUND_THRESHOLD]);
            Self::render_weight_row(ui, "lookahead", w[HEURISTIC_LOOKAHEAD]);
            Self::render_weight_row(ui, "progressive_bias", w[PROGRESSIVE_BIAS_WEIGHT]);
        });
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
    fn show_output(&mut self, _pin: &OutPin, ui: &mut egui::Ui, _snarl: &mut Snarl<DiffEvalNode>) -> PinInfo {
        ui.label("out");
        PinInfo::circle().with_fill(egui::Color32::from_rgb(200, 150, 150))
    }

    fn has_body(&mut self, _node: &DiffEvalNode) -> bool {
        true
    }

    fn show_body(
        &mut self,
        node: NodeId,
        _inputs: &[InPin],
        _outputs: &[OutPin],
        ui: &mut egui::Ui,
        snarl: &mut Snarl<DiffEvalNode>,
    ) {
        let node_data = &snarl[node];
        match node_data {
            DiffEvalNode::ColorWheelValue => self.render_color_wheel_body(ui),
            DiffEvalNode::SellCardAlignment => self.render_sell_card_body(ui),
            DiffEvalNode::DeckColorProfile => self.render_deck_profile_body(ui),
            DiffEvalNode::MaterialStrategy => self.render_material_body(ui),
            DiffEvalNode::HiddenLayer => self.render_hidden_layer_body(ui),
            DiffEvalNode::OutputLayer => self.render_output_layer_body(ui),
            DiffEvalNode::WinProbability => self.render_control_params(ui),
            DiffEvalNode::InputScore => { ui.label("Player's cached score / 20"); }
            DiffEvalNode::InputColorWheel => { ui.label("12 color counts"); }
            DiffEvalNode::InputMaterials => { ui.label("3 material counts"); }
            DiffEvalNode::InputSellCards => { ui.label("6 sell cards in display"); }
            DiffEvalNode::InputDeck => { ui.label("All cards in player's deck"); }
            DiffEvalNode::InputRound => { ui.label("Current round / 20"); }
        }
    }

    fn connect(&mut self, _from: &OutPin, _to: &InPin, _snarl: &mut Snarl<DiffEvalNode>) {
        // Read-only: don't allow new connections
    }

    fn disconnect(&mut self, _from: &OutPin, _to: &InPin, _snarl: &mut Snarl<DiffEvalNode>) {
        // Read-only: don't allow disconnections
    }
}

// ── Graph construction ──

fn build_graph() -> Snarl<DiffEvalNode> {
    let mut snarl = Snarl::new();

    let col_input = 0.0;
    let col_module = 350.0;
    let col_mlp = 750.0;
    let col_output = 1100.0;

    // Input nodes
    let score_id = snarl.insert_node(egui::pos2(col_input, 0.0), DiffEvalNode::InputScore);
    let color_id = snarl.insert_node(egui::pos2(col_input, 60.0), DiffEvalNode::InputColorWheel);
    let mat_id = snarl.insert_node(egui::pos2(col_input, 120.0), DiffEvalNode::InputMaterials);
    let sell_id = snarl.insert_node(egui::pos2(col_input, 180.0), DiffEvalNode::InputSellCards);
    let deck_id = snarl.insert_node(egui::pos2(col_input, 240.0), DiffEvalNode::InputDeck);
    let round_id = snarl.insert_node(egui::pos2(col_input, 300.0), DiffEvalNode::InputRound);

    // Module nodes
    let cwv_id = snarl.insert_node(egui::pos2(col_module, 0.0), DiffEvalNode::ColorWheelValue);
    let sca_id = snarl.insert_node(egui::pos2(col_module, 200.0), DiffEvalNode::SellCardAlignment);
    let dcp_id = snarl.insert_node(egui::pos2(col_module, 500.0), DiffEvalNode::DeckColorProfile);
    let ms_id = snarl.insert_node(egui::pos2(col_module, 750.0), DiffEvalNode::MaterialStrategy);

    // MLP nodes
    let hidden_id = snarl.insert_node(egui::pos2(col_mlp, 100.0), DiffEvalNode::HiddenLayer);
    let output_id = snarl.insert_node(egui::pos2(col_mlp, 600.0), DiffEvalNode::OutputLayer);

    // Output node
    let win_id = snarl.insert_node(egui::pos2(col_output, 300.0), DiffEvalNode::WinProbability);

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

    // Module -> Hidden Layer
    snarl.connect(OutPinId { node: score_id, output: 0 }, InPinId { node: hidden_id, input: 0 });
    snarl.connect(OutPinId { node: cwv_id, output: 0 }, InPinId { node: hidden_id, input: 1 });
    snarl.connect(OutPinId { node: sca_id, output: 0 }, InPinId { node: hidden_id, input: 2 });
    snarl.connect(OutPinId { node: dcp_id, output: 0 }, InPinId { node: hidden_id, input: 3 });
    snarl.connect(OutPinId { node: ms_id, output: 0 }, InPinId { node: hidden_id, input: 4 });
    snarl.connect(OutPinId { node: round_id, output: 0 }, InPinId { node: hidden_id, input: 5 });

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
