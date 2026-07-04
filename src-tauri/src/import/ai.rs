use anyhow::Result;
use serde::{Deserialize, Serialize};
use reqwest::Client;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;
use tokio::sync::watch;
use crate::import::structure::{ParsedQuestion, extract_answers_from_text, normalize_answer, detect_type};

#[derive(Serialize)]
struct AiMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Serialize)]
struct AiRequest<'a> {
    model: &'a str,
    messages: Vec<AiMessage<'a>>,
    temperature: f32,
    max_tokens: u32,
}

#[derive(Deserialize)]
struct AiResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: RespMessage,
}

#[derive(Deserialize)]
struct RespMessage {
    content: String,
}

const PROMPT: &str = r#"你是题库结构化助手。下面是 Word 文档提取的纯文本内容（部分题目）。
请识别其中所有题目，输出 JSON 数组，每个元素格式：
{"index":题号,"type":"single|multi|judge|blank|qa","stem":"题干","options":["A选项","B选项",...],"answer":"答案"}

规则：
- index 为题目的原始题号（整数），必须从文档中读取，不要自己编造
- type 取值：single(单选)、multi(多选)、judge(判断)、blank(填空)、qa(问答)
- 单选 answer 必须为单个大写字母如 "A"，绝不能返回多个字母
- 多选 answer 为字母串如 "AC"，不要用数组或分隔符
- 判断为 "true" 或 "false"；填空/问答为答案文本
- options 仅对选择题填写，其他类型留空数组 []
- 若本段文本中没有答案，answer 填 null
- 不要输出 analysis 字段，只输出上述字段
- 只输出 JSON 数组，不要任何解释文字、不要 markdown 代码块标记
"#;

/// 每块的题目数上限
/// 15 题/块（无 analysis 字段）：输出约 800-1500 token，max_tokens=2048 足够
/// 较小分块 = 单次请求输出更少 = 响应时间更短 + 触达 token 上限概率更低
const QUESTIONS_PER_CHUNK: usize = 15;
/// 失败块重试次数
const MAX_RETRIES: usize = 2;

/// 单次 AI 请求的最大输出 token
/// 选择题为主的题库，单题 JSON 约 50-100 token，15 题约 1500，留 2048 余量
/// 比 4096 节省约 50% 输出时间（output token 与响应时间近似线性）
const MAX_TOKENS: u32 = 2048;

/// 根据 base_url 自动检测最佳并发数
/// - agnes-ai.com: 16（免费高并发）
/// - bigmodel.cn（智谱）: 6（免费版 QPS=5 略高一点靠重试兜底）
/// - deepseek.com: 8
/// - openai.com: 10
/// - 默认: 6（安全值）
fn detect_concurrency(base_url: &str) -> usize {
    let url = base_url.to_lowercase();
    if url.contains("agnes-ai.com") { 16 }
    else if url.contains("bigmodel.cn") { 6 }
    else if url.contains("deepseek.com") { 8 }
    else if url.contains("openai.com") { 10 }
    else { 6 }
}

pub async fn ai_structurize(
    text: &str,
    api_key: &str,
    base_url: &str,
    model: &str,
    progress_tx: Option<watch::Sender<(usize, usize)>>,
    cancel: Arc<AtomicBool>,
) -> Result<Vec<ParsedQuestion>> {
    // 1. 本地提取全局答案表（文末答案表）
    let mut answer_map: HashMap<usize, String> = HashMap::new();
    for line in text.lines() {
        extract_answers_from_text(line, &mut answer_map, None);
    }
    eprintln!("[AI] 本地答案表提取到 {} 条答案", answer_map.len());

    // 2. 按题号边界分块
    let chunks = split_into_chunks(text, QUESTIONS_PER_CHUNK);
    let total_chunks = chunks.len();
    let expected_count = count_question_boundaries(text);
    eprintln!("[AI] 文档分为 {} 块，每块约 {} 题，预计 {} 题", total_chunks, QUESTIONS_PER_CHUNK, expected_count);

    if chunks.is_empty() {
        return Err(anyhow::anyhow!("文档为空或无法识别题目"));
    }

    // 通知前端总块数
    if let Some(ref tx) = progress_tx {
        tx.send((0, total_chunks)).ok();
    }

    // 3. 并发调用 AI（信号量控制并发数）
    // connect_timeout: 连接阶段 15s 超时（防止 API 不可达时无限挂起）
    // timeout: 整个请求 90s 超时
    let concurrency = detect_concurrency(base_url);
    eprintln!("[AI] 并发数: {}（根据 {} 检测）", concurrency, base_url);
    let client = Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(90))
        // 复用 HTTP 连接，避免每个请求都重新 TCP/TLS 握手（节省 100-300ms/请求）
        .pool_max_idle_per_host(concurrency * 2)
        .tcp_keepalive(Duration::from_secs(30))
        .build()?;
    let semaphore = Arc::new(tokio::sync::Semaphore::new(concurrency));
    let mut handles = Vec::new();
    let completed = Arc::new(AtomicUsize::new(0));
    let start_time = std::time::Instant::now();

    for (i, chunk) in chunks.iter().enumerate() {
        let client = client.clone();
        let api_key = api_key.to_string();
        let base_url = base_url.to_string();
        let model = model.to_string();
        let chunk = chunk.clone();
        let permit = semaphore.clone();
        let completed_clone = completed.clone();
        let tx_clone = progress_tx.clone();
        let cancel_clone = cancel.clone();
        handles.push(tokio::spawn(async move {
            let _p = permit.acquire_owned().await.map_err(|e| anyhow::anyhow!("信号量错误: {}", e))?;
            // 取消检查：用户点取消后，未开始的块直接跳过
            if cancel_clone.load(Ordering::Relaxed) {
                return Ok(Vec::new());
            }
            let chunk_start = std::time::Instant::now();
            eprintln!("[AI] 第 {}/{} 块开始（{}字符）", i + 1, total_chunks, chunk.chars().count());
            // 重试机制：失败块自动重试，避免网络抖动/AI 偶发错误导致整块丢题
            let mut last_err = None;
            let mut qs: Vec<ParsedQuestion> = Vec::new();
            for attempt in 1..=MAX_RETRIES {
                if cancel_clone.load(Ordering::Relaxed) {
                    return Ok(Vec::new());
                }
                match call_ai(&client, &chunk, &api_key, &base_url, &model).await {
                    Ok(result) => {
                        qs = result;
                        if attempt > 1 {
                            eprintln!("[AI] 第 {}/{} 块第 {} 次尝试成功", i + 1, total_chunks, attempt);
                        }
                        last_err = None;
                        break;
                    }
                    Err(e) => {
                        let err_str = e.to_string();
                        eprintln!("[AI] 第 {}/{} 块第 {}/{} 次尝试失败: {}", i + 1, total_chunks, attempt, MAX_RETRIES, e);
                        last_err = Some(e);
                        // 429 限流：等待 5 秒再重试；其他错误等 1 秒
                        let backoff = if err_str.contains("429") || err_str.contains("Too Many Requests") || err_str.contains("rate") {
                            eprintln!("[AI] 检测到限流，等待 5 秒后重试");
                            5
                        } else {
                            1
                        };
                        tokio::time::sleep(Duration::from_secs(backoff)).await;
                    }
                }
            }
            if let Some(e) = last_err {
                let c = completed_clone.fetch_add(1, Ordering::Relaxed) + 1;
                if let Some(tx) = tx_clone { tx.send((c, total_chunks)).ok(); }
                return Err(anyhow::anyhow!("块{} 重试{}次仍失败: {}", i + 1, MAX_RETRIES, e));
            }
            let c = completed_clone.fetch_add(1, Ordering::Relaxed) + 1;
            eprintln!("[AI] 第 {}/{} 块完成，识别 {} 题，耗时 {:.1}s", i + 1, total_chunks, qs.len(), chunk_start.elapsed().as_secs_f64());
            if let Some(tx) = tx_clone {
                tx.send((c, total_chunks)).ok();
            }
            Ok::<Vec<ParsedQuestion>, anyhow::Error>(qs)
        }));
    }

    // 4. 收集结果（按块顺序）
    let mut all_questions = Vec::new();
    let mut errors = Vec::new();
    let mut succeeded_chunks = 0usize;
    let mut consecutive_failures = 0usize;
    let mut aborted = false;
    for (i, h) in handles.into_iter().enumerate() {
        // 取消检查
        if cancel.load(Ordering::Relaxed) {
            aborted = true;
            break;
        }
        match h.await {
            Ok(Ok(qs)) => {
                all_questions.extend(qs);
                succeeded_chunks += 1;
                consecutive_failures = 0;
            }
            Ok(Err(e)) => {
                errors.push(format!("块{}: {}", i + 1, e));
                consecutive_failures += 1;
                // 快速失败：前 3 块全部失败 → API 不可用，立即中止
                if consecutive_failures >= 3 && succeeded_chunks == 0 {
                    eprintln!("[AI] 前 {} 块全部失败，快速中止", consecutive_failures);
                    cancel.store(true, Ordering::Relaxed);
                    aborted = true;
                    break;
                }
            }
            Err(e) => {
                errors.push(format!("块{} 任务异常: {}", i + 1, e));
                consecutive_failures += 1;
                if consecutive_failures >= 3 && succeeded_chunks == 0 {
                    cancel.store(true, Ordering::Relaxed);
                    aborted = true;
                    break;
                }
            }
        }
    }
    let failed_chunks = total_chunks - succeeded_chunks;
    eprintln!("[AI] 识别完成{}：成功 {}/{} 块，失败 {} 块，共识别 {} 题（预计 {} 题），总耗时 {:.1}s",
        if aborted { "（已中止）" } else { "" },
        succeeded_chunks, total_chunks, failed_chunks, all_questions.len(), expected_count, start_time.elapsed().as_secs_f64());
    if !errors.is_empty() {
        eprintln!("[AI] 失败块详情：{}", errors.join("; "));
    }
    if all_questions.is_empty() && !errors.is_empty() {
        if aborted {
            return Err(anyhow::anyhow!("AI 接口连续失败，已中止。请检查：1) API Key 是否正确 2) 网络是否通畅 3) API 地址是否可达。错误：{}", errors.join("; ")));
        }
        return Err(anyhow::anyhow!("所有块均失败：{}", errors.join("; ")));
    }
    if aborted {
        eprintln!("[AI] 导入被中止，已识别 {} 题", all_questions.len());
    }

    // 5. 用本地答案表回填 AI 未识别的答案
    let mut filled = 0;
    for q in all_questions.iter_mut() {
        let need = match &q.answer {
            None => true,
            Some(a) => a.trim().is_empty(),
        };
        if need {
            if let Some(a) = answer_map.get(&q.source_index) {
                q.answer = Some(normalize_answer(a));
                filled += 1;
            }
        }
    }
    eprintln!("[AI] 答案回填 {} 题，共 {} 题", filled, all_questions.len());

    // 6. 校正选择题类型与答案（AI 可能将单选题答案返回为多字母，或返回 JSON 数组字符串）
    let corrected = post_process_questions(&mut all_questions);
    eprintln!("[AI] 类型校正 {} 题（共 {} 题）", corrected, all_questions.len());

    Ok(all_questions)
}

/// 校正选择题类型与答案。
/// - 归一化所有选择题答案（AI 可能返回 "ABCD" 或 `["A","B","C","D"]` 等格式）
/// - 根据答案字母数重新检测单选/多选（单选题答案不应有多个字母）
/// - 校正判断题：AI 可能把判断题分成 blank/qa，但 answer=true/false 应为 judge
fn post_process_questions(questions: &mut [ParsedQuestion]) -> usize {
    let mut corrected = 0;
    for q in questions.iter_mut() {
        if q.options.is_empty() {
            // 非选择题：重新检测类型（判断题 answer=true/false 但被分成 blank 的情况）
            let detected = detect_type(&q.stem, &q.options, &q.answer);
            if detected != q.q_type {
                q.q_type = detected;
                corrected += 1;
            }
            continue;
        }
        // 归一化答案
        if let Some(a) = q.answer.take() {
            q.answer = Some(normalize_answer(&a));
        }
        // 按答案字母数重新检测单选/多选
        let detected = detect_type(&q.stem, &q.options, &q.answer);
        if detected != q.q_type {
            q.q_type = detected;
            corrected += 1;
        }
    }
    corrected
}

/// 调用 AI 解析单题：解释题干、选项、正确答案与解题思路
pub async fn analyze_question(
    stem: &str,
    q_type: &str,
    options: &[String],
    answer: Option<&str>,
    api_key: &str,
    base_url: &str,
    model: &str,
) -> Result<String> {
    let type_label = match q_type {
        "single" => "单选题",
        "multi" => "多选题",
        "judge" => "判断题",
        "blank" => "填空题",
        "qa" => "问答题",
        other => other,
    };

    let mut user = String::new();
    user.push_str(&format!("题型：{}\n", type_label));
    user.push_str(&format!("题干：{}\n", stem));
    if !options.is_empty() {
        user.push_str("选项：\n");
        for (i, opt) in options.iter().enumerate() {
            let letter = (b'A' + i as u8) as char;
            user.push_str(&format!("{}. {}\n", letter, opt));
        }
    }
    if let Some(a) = answer {
        user.push_str(&format!("参考答案：{}\n", a));
    } else {
        user.push_str("参考答案：（无）\n");
    }

    // 结构化输出 prompt：让 AI 直接返回 JSON 对象
    // 前端按字段渲染（知识点 / 选项解析 / 参考答案 / 解题思路），不再依赖 markdown 文本
    let system = r#"你是一位经验丰富的题目解析老师。请对给出的题目进行**详细解析**，**只输出一个 JSON 对象**，格式如下：

{
  "knowledge_point": "考查的知识点（说明属于哪个学科领域、什么主题，2-3 句话，必要时引用相关概念或原理）",
  "background": "相关背景知识（解释题目涉及的概念、原理、历史背景或现实意义，帮助用户理解为什么这么考，3-5 句话）",
  "option_analysis": [
    {"letter": "A", "verdict": "正确"或"错误", "reason": "该选项为什么对/错，结合相关知识点详细说明，2-3 句话"},
    {"letter": "B", "verdict": "...", "reason": "..."},
    {"letter": "C", "verdict": "...", "reason": "..."},
    {"letter": "D", "verdict": "...", "reason": "..."}
  ],
  "reference_explanation": "为什么参考答案是正确的（结合背景知识和题干关键信息，引用相关原理/概念，详细说明判断依据，3-5 句话）",
  "common_mistakes": "常见错误（考生在此题上容易选错的选项及原因，1-2 句话）",
  "solving_skill": "此类题目的通用解题技巧（包含识别题型的方法、解题步骤、记忆口诀或易混点对比，3-5 句话，要实用好记）"
}

要求：
1. 严格 JSON，不要任何解释文字、不要 markdown 代码块标记
2. 选项解析只针对选择题（single/multi），其他题型 option_analysis 留空数组 []
3. **内容要详细充实**：每个字段都要有实质内容，不要一两句话草草了事
4. 举例说明、数据支撑、对比分析都可以用上
5. 用中文回答"#;
    let user_tail = "\n请按要求输出详细 JSON 解析。";
    user.push_str(user_tail);

    let client = Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(120))
        .pool_max_idle_per_host(4)
        .tcp_keepalive(Duration::from_secs(30))
        .build()?;
    let req = AiRequest {
        model,
        messages: vec![
            AiMessage { role: "system", content: system },
            AiMessage { role: "user", content: &user },
        ],
        temperature: 0.3,
        max_tokens: 3000,  // 详细解析 6 个字段需要 2500-3000 token，单题解析可以慢一点但要详尽
    };

    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));
    let resp = client.post(&url)
        .bearer_auth(api_key)
        .json(&req)
        .send().await
        .map_err(|e| anyhow::anyhow!("请求失败: {}", e))?;

    let status = resp.status();
    let body = resp.text().await.map_err(|e| anyhow::anyhow!("读取响应失败: {}", e))?;
    if !status.is_success() {
        let preview: String = body.chars().take(800).collect();
        return Err(anyhow::anyhow!("HTTP {}: {}", status, preview));
    }

    let ai_resp: AiResponse = serde_json::from_str(&body)
        .map_err(|e| {
            let preview: String = body.chars().take(800).collect();
            anyhow::anyhow!("响应解析失败: {} | {}", e, preview)
        })?;
    let content = ai_resp.choices.into_iter().next()
        .ok_or_else(|| anyhow::anyhow!("无 choices"))?
        .message.content;
    Ok(content)
}

/// 测试 AI 连通性：发送一个最小请求，快速验证 API Key、地址、模型是否可用
pub async fn test_connection(api_key: &str, base_url: &str, model: &str) -> Result<()> {
    let client = Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .build()?;
    let req = AiRequest {
        model,
        messages: vec![
            AiMessage { role: "user", content: "hi" },
        ],
        temperature: 0.0,
        max_tokens: 8,
    };
    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));
    let resp = client.post(&url)
        .bearer_auth(api_key)
        .json(&req)
        .send().await
        .map_err(|e| anyhow::anyhow!("连接失败：{}（请检查 API 地址和网络）", e))?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        let preview: String = body.chars().take(300).collect();
        return Err(anyhow::anyhow!("API 返回错误 HTTP {}：{}（请检查 API Key 和模型名）", status, preview));
    }
    Ok(())
}

/// 调用 AI 识别单块文本
async fn call_ai(client: &Client, text: &str, api_key: &str, base_url: &str, model: &str) -> Result<Vec<ParsedQuestion>> {
    let req = AiRequest {
        model,
        messages: vec![
            AiMessage { role: "system", content: PROMPT },
            AiMessage { role: "user", content: text },
        ],
        temperature: 0.1,
        max_tokens: MAX_TOKENS,
    };

    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));
    let resp = client.post(&url)
        .bearer_auth(api_key)
        .json(&req)
        .send().await
        .map_err(|e| anyhow::anyhow!("请求失败: {}", e))?;

    let status = resp.status();
    let body = resp.text().await.map_err(|e| anyhow::anyhow!("读取响应失败: {}", e))?;

    if !status.is_success() {
        let preview: String = body.chars().take(800).collect();
        return Err(anyhow::anyhow!("HTTP {}: {}", status, preview));
    }

    let ai_resp: AiResponse = serde_json::from_str(&body)
        .map_err(|e| {
            let preview: String = body.chars().take(800).collect();
            anyhow::anyhow!("响应解析失败: {} | {}", e, preview)
        })?;

    let content = ai_resp.choices.into_iter().next()
        .ok_or_else(|| anyhow::anyhow!("无 choices"))?
        .message.content;

    let json = extract_json(&content);
    // 先尝试完整解析
    let parsed_result = serde_json::from_str::<Vec<ParsedQuestion>>(&json);
    let mut parsed = match parsed_result {
        Ok(qs) => qs,
        Err(e) => {
            // 完整解析失败 → 尝试抢救截断的 JSON（AI 输出被 max_tokens 截断时常见）
            let preview: String = json.chars().take(300).collect();
            eprintln!("[AI] JSON 解析失败，尝试抢救截断响应... 错误: {} | 开头: {}", e, preview);
            match salvage_json_array(&json) {
                Some(salvaged) => {
                    let qs: Vec<ParsedQuestion> = serde_json::from_str(&salvaged)
                        .map_err(|e2| anyhow::anyhow!("JSON 抢救后仍解析失败: {}", e2))?;
                    eprintln!("[AI] 抢救成功，恢复 {} 题", qs.len());
                    qs
                }
                None => {
                    let preview: String = json.chars().take(800).collect();
                    return Err(anyhow::anyhow!("题目JSON解析失败且无法抢救: {} | {}", e, preview));
                }
            }
        }
    };

    for pq in parsed.iter_mut() {
        pq.confidence = 0.9;
    }
    Ok(parsed)
}

/// 按题号边界分块。每块包含不超过 max_questions 个题目。
fn split_into_chunks(text: &str, max_questions: usize) -> Vec<String> {
    let lines: Vec<&str> = text.lines().collect();
    // 题号行：行首数字后跟 . 、 ． ) ） 或 (1) （1） 格式
    let re_num = regex::Regex::new(r"^\s*[\(（]?\d+[\.、．)）]\s*").unwrap();

    // 找所有题号行的行索引
    let mut boundaries: Vec<usize> = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        if re_num.is_match(line) {
            boundaries.push(i);
        }
    }

    if boundaries.is_empty() {
        // 无题号，按字符数分块（每块 8000 字符）
        return split_by_chars(text, 8000);
    }

    let mut chunks = Vec::new();
    let mut start = 0usize;
    while start < boundaries.len() {
        let end = (start + max_questions).min(boundaries.len());
        let line_start = boundaries[start];
        let line_end = if end < boundaries.len() {
            boundaries[end]
        } else {
            lines.len()
        };
        let chunk: String = lines[line_start..line_end].join("\n");
        if !chunk.trim().is_empty() {
            chunks.push(chunk);
        }
        start = end;
    }
    chunks
}

/// 无题号时按字符数分块
fn split_by_chars(text: &str, max_chars: usize) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut chunks = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let end = (i + max_chars).min(chars.len());
        let s: String = chars[i..end].iter().collect();
        if !s.trim().is_empty() {
            chunks.push(s);
        }
        i = end;
    }
    if chunks.is_empty() {
        chunks.push(text.to_string());
    }
    chunks
}

/// 从 AI 输出中提取 JSON 数组（兼容 markdown 代码块包裹）
fn extract_json(s: &str) -> String {
    let s = s.trim();
    let s = s.strip_prefix("```json").or_else(|| s.strip_prefix("```")).unwrap_or(s);
    let s = s.strip_suffix("```").unwrap_or(s);
    let s = s.trim();
    if let Some(start) = s.find('[') {
        if let Some(end) = s.rfind(']') {
            return s[start..=end].to_string();
        }
    }
    s.to_string()
}

/// 统计文本中的题号行数量（用于预估总题数，和识别结果对比）
pub fn count_question_boundaries(text: &str) -> usize {
    let re_num = regex::Regex::new(r"^\s*[\(（]?\d+[\.、．)）]\s*").unwrap();
    text.lines().filter(|line| re_num.is_match(line)).count()
}

/// 抢救截断的 JSON 数组：AI 输出被 max_tokens 截断时，JSON 不完整。
/// 找到最后一个完整的 `}`，截断后补 `]` 闭合数组，尽量恢复已输出的题目。
fn salvage_json_array(s: &str) -> Option<String> {
    let s = s.trim();
    let s = s.strip_prefix("```json").or_else(|| s.strip_prefix("```")).unwrap_or(s);
    let s = s.strip_suffix("```").unwrap_or(s);
    let s = s.trim();

    let start = s.find('[')?;
    let subset = &s[start..];

    // 找到最后一个完整的对象结尾 `}`
    if let Some(last_brace) = subset.rfind('}') {
        let candidate = format!("{}]", &subset[..=last_brace]);
        // 验证可解析
        if serde_json::from_str::<serde_json::Value>(&candidate).is_ok() {
            return Some(candidate);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::import::structure::QType;

    #[test]
    fn test_extract_json_plain() {
        let s = r#"[{"index":1,"type":"single","stem":"题","options":["A"],"answer":"A","analysis":""}]"#;
        let j = extract_json(s);
        assert!(j.starts_with('[') && j.ends_with(']'));
    }

    #[test]
    fn test_extract_json_with_codeblock() {
        let s = "```json\n[{\"index\":1,\"type\":\"single\",\"stem\":\"x\",\"options\":[],\"answer\":\"A\",\"analysis\":\"\"}]\n```";
        let j = extract_json(s);
        assert!(j.starts_with('[') && j.ends_with(']'));
    }

    #[test]
    fn test_split_into_chunks_by_question() {
        let text = "1. 题1\nA. a\n2. 题2\nB. b\n3. 题3\nC. c\n4. 题4\nD. d";
        let chunks = split_into_chunks(text, 2);
        assert_eq!(chunks.len(), 2);
        assert!(chunks[0].contains("题1") && chunks[0].contains("题2"));
        assert!(chunks[1].contains("题3") && chunks[1].contains("题4"));
    }

    #[test]
    fn test_split_by_chars_fallback() {
        let text = "无题号的内容".repeat(2000); // 12000 字符 > 8000
        let chunks = split_into_chunks(&text, 8000);
        assert!(chunks.len() > 1);
    }

    // ===== post_process_questions 测试 =====

    fn make_question(q_type: QType, options: Vec<&str>, answer: Option<&str>) -> ParsedQuestion {
        ParsedQuestion {
            q_type,
            stem: "题干".to_string(),
            options: options.iter().map(|s| s.to_string()).collect(),
            answer: answer.map(|s| s.to_string()),
            analysis: None,
            source_index: 1,
            confidence: 0.9,
        }
    }

    #[test]
    fn test_single_with_multi_letter_answer_corrected_to_multi() {
        // AI 返回 single + "ABCD" → 应校正为 Multi
        let mut qs = vec![make_question(QType::Single, vec!["甲", "乙", "丙", "丁"], Some("ABCD"))];
        let corrected = post_process_questions(&mut qs);
        assert_eq!(corrected, 1);
        assert_eq!(qs[0].q_type, QType::Multi);
        assert_eq!(qs[0].answer.as_deref(), Some(r#"["A","B","C","D"]"#));
    }

    #[test]
    fn test_single_with_single_letter_unchanged() {
        let mut qs = vec![make_question(QType::Single, vec!["甲", "乙", "丙", "丁"], Some("A"))];
        let corrected = post_process_questions(&mut qs);
        assert_eq!(corrected, 0);
        assert_eq!(qs[0].q_type, QType::Single);
        assert_eq!(qs[0].answer.as_deref(), Some("A"));
    }

    #[test]
    fn test_multi_with_multi_letter_unchanged() {
        let mut qs = vec![make_question(QType::Multi, vec!["甲", "乙", "丙", "丁"], Some("AC"))];
        let corrected = post_process_questions(&mut qs);
        assert_eq!(corrected, 0);
        assert_eq!(qs[0].q_type, QType::Multi);
        assert_eq!(qs[0].answer.as_deref(), Some(r#"["A","C"]"#));
    }

    #[test]
    fn test_multi_with_single_letter_corrected_to_single() {
        // 多选题但答案只有一个字母 → 校正为单选
        let mut qs = vec![make_question(QType::Multi, vec!["甲", "乙", "丙", "丁"], Some("B"))];
        let corrected = post_process_questions(&mut qs);
        assert_eq!(corrected, 1);
        assert_eq!(qs[0].q_type, QType::Single);
        assert_eq!(qs[0].answer.as_deref(), Some("B"));
    }

    #[test]
    fn test_non_choice_question_skipped() {
        // 问答题无选项，不应被处理
        let mut qs = vec![make_question(QType::Qa, vec![], Some("FTP是文件传输协议"))];
        let corrected = post_process_questions(&mut qs);
        assert_eq!(corrected, 0);
        assert_eq!(qs[0].q_type, QType::Qa);
        assert_eq!(qs[0].answer.as_deref(), Some("FTP是文件传输协议"));
    }

    #[test]
    fn test_answer_json_array_string_normalized() {
        // AI 返回 JSON 数组字符串作为答案
        let mut qs = vec![make_question(QType::Multi, vec!["甲", "乙", "丙", "丁"], Some(r#"["A","C"]"#))];
        let corrected = post_process_questions(&mut qs);
        assert_eq!(corrected, 0);
        assert_eq!(qs[0].q_type, QType::Multi);
        // 归一化后仍为标准格式
        assert_eq!(qs[0].answer.as_deref(), Some(r#"["A","C"]"#));
    }
}
