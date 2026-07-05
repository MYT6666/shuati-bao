// 端到端验证：用真实 客观题.docx 提取的段落跑实际 parse_questions，统计识别与匹配情况。
// 运行：cargo run --example verify_docx（需先在项目根生成 _extracted.txt）
use std::collections::HashMap;
use tauri_app_lib::import::structure::{parse_questions, QType};

fn main() {
    let text = std::fs::read_to_string("_extracted.txt")
        .or_else(|_| std::fs::read_to_string("../_extracted.txt"))
        .or_else(|_| std::fs::read_to_string("../../_extracted.txt"))
        .expect("无法读取 _extracted.txt，请先在项目根生成该文件");

    let paragraphs: Vec<String> = text.lines().map(|s| s.to_string()).collect();
    println!("输入段落总数: {}", paragraphs.len());

    let qs = parse_questions(&paragraphs);
    let total = qs.len();
    let with_ans = qs.iter().filter(|q| q.answer.is_some()).count();
    let without_ans = total - with_ans;

    println!("\n========== 整体统计 ==========");
    println!("识别题目总数: {}", total);
    println!("有答案匹配: {} ({:.1}%)", with_ans, with_ans as f64 / total as f64 * 100.0);
    println!("无答案匹配: {} ({:.1}%)", without_ans, without_ans as f64 / total as f64 * 100.0);

    // 题型分布
    let mut types: HashMap<&str, usize> = HashMap::new();
    for q in &qs {
        let t = match q.q_type {
            QType::Single => "单选",
            QType::Multi => "多选",
            QType::Judge => "判断",
            QType::Blank => "填空",
            QType::Qa => "问答",
        };
        *types.entry(t).or_default() += 1;
    }
    println!("\n========== 题型分布 ==========");
    for t in ["单选", "多选", "判断", "填空", "问答"] {
        if let Some(&c) = types.get(t) {
            let with = qs.iter().filter(|q| q_type_name(&q.q_type) == t && q.answer.is_some()).count();
            println!("  {}: {} 题 (有答案 {} / {:.1}%)", t, c, with, with as f64 / c as f64 * 100.0);
        }
    }

    // 按章节统计（通过 source_index 范围无法直接得章节，这里用题号位置近似）
    // 改为统计无答案样本
    println!("\n========== 无答案样本 (前 25) ==========");
    let mut shown = 0;
    for q in qs.iter().filter(|q| q.answer.is_none()) {
        if shown >= 25 { break; }
        let stem_head: String = q.stem.chars().take(70).collect();
        let opts_head = if q.options.is_empty() {
            String::from("(无选项)")
        } else {
            format!("[{} 个选项]", q.options.len())
        };
        println!("  src={} {} {}", q.source_index, opts_head, stem_head);
        shown += 1;
    }

    // 选项数异常检测（选择题应有 2-6 个选项）
    println!("\n========== 选项数异常检测 ==========");
    let mut bad_opts = 0;
    for q in &qs {
        if matches!(q.q_type, QType::Single | QType::Multi) {
            if q.options.len() < 2 || q.options.len() > 6 {
                if bad_opts < 10 {
                    println!("  src={} type={:?} opts={} stem={:?}",
                        q.source_index, q.q_type, q.options.len(),
                        q.stem.chars().take(50).collect::<String>());
                }
                bad_opts += 1;
            }
        }
    }
    println!("选项数异常题数: {}", bad_opts);

    // 多选答案格式校验
    println!("\n========== 多选答案格式样本 ==========");
    let mut shown = 0;
    for q in qs.iter().filter(|q| matches!(q.q_type, QType::Multi) && q.answer.is_some()) {
        if shown >= 8 { break; }
        println!("  src={} ans={:?} stem={:?}",
            q.source_index, q.answer.as_deref().unwrap_or(""),
            q.stem.chars().take(40).collect::<String>());
        shown += 1;
    }

    // 判断题答案归一化校验
    println!("\n========== 判断题答案归一化校验 ==========");
    let mut bad_judge = 0;
    for q in qs.iter().filter(|q| matches!(q.q_type, QType::Judge)) {
        if let Some(a) = &q.answer {
            if a != "true" && a != "false" {
                if bad_judge < 10 {
                    println!("  未归一化: src={} ans={:?} stem={:?}",
                        q.source_index, a, q.stem.chars().take(40).collect::<String>());
                }
                bad_judge += 1;
            }
        }
    }
    println!("未归一化的判断题答案数: {}", bad_judge);

    println!("\n========== 验证完成 ==========");
}

fn q_type_name(t: &QType) -> &'static str {
    match t {
        QType::Single => "单选",
        QType::Multi => "多选",
        QType::Judge => "判断",
        QType::Blank => "填空",
        QType::Qa => "问答",
    }
}
