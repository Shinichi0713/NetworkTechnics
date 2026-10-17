use std::collections::HashMap;

// --- 1. Message 構造体 ---
#[derive(Debug, Clone)]
struct Message {
    role: String,    // "User" または "Assistant"
    content: String,
    timestamp: String,
}

impl Message {
    fn new(role: &str, content: &str) -> Self {
        // 簡易的なタイムスタンプ（実運用ではchronoクレイト等を使用可能ですが標準機能で代用）
        let timestamp = "2026-10-15T00:00:00".to_string();
        Self {
            role: role.to_string(),
            content: content.to_string(),
            timestamp,
        }
    }
}

// --- 2. MemoryManager 構造体 ---
struct MemoryManager {
    short_term: Vec<Message>,
    short_term_limit: usize,
    long_term: HashMap<String, String>,
}

impl MemoryManager {
    fn new(short_term_limit: usize) -> Self {
        Self {
            short_term: Vec::new(),
            short_term_limit,
            long_term: HashMap::new(),
        }
    }

    /// 短期記憶にメッセージを追加し、上限を超えたら古いものを削除
    fn add_short_term(&mut self, role: &str, content: &str) {
        self.short_term.push(Message::new(role, content));
        if self.short_term.len() > self.short_term_limit {
            self.short_term.remove(0); // 最も古い履歴を削除
        }
    }

    /// 長期記憶に重要な知識や属性を保存
    fn add_long_term(&mut self, key: &str, value: &str) {
        self.long_term.insert(key.to_string(), value.to_string());
    }

    /// クエリに関連する長期記憶をキーワードマッチで検索
    fn search_long_term(&self, query: &str) -> Vec<String> {
        let mut results = Vec::new();
        let query_lower = query.to_lowercase();

        for (key, value) in &self.long_term {
            if query_lower.contains(&key.to_lowercase())
                || query_lower.contains(&value.to_lowercase())
                || query_lower.contains("すべて")
            {
                results.push(format!("【記憶 - {}】: {}", key, value));
            }
        }
        results
    }

    /// プロンプトに渡すためのコンテキスト文字列を構築
    fn build_context(&self, current_query: &str) -> String {
        let mut context_parts = Vec::new();

        // 長期記憶から関連情報を検索
        let relevant_memories = self.search_long_term(current_query);
        if !relevant_memories.empty_or_custom(&relevant_memories) {
            context_parts.push("--- [長期記憶・ユーザープロファイル] ---".to_string());
            context_parts.extend(relevant_memories);
        }

        // 短期記憶（直近の対話履歴）を付与
        if !self.short_term.is_empty() {
            context_parts.push("--- [直近の会話履歴] ---".to_string());
            for msg in &self.short_term {
                context_parts.push(format!("{}: {}", msg.role, msg.content));
            }
        }

        context_parts.join("\n")
    }
}

// 補助トレイトまたはメソッド
trait VecExt {
    fn empty_or_custom(&self, v: &Vec<String>) -> bool;
}
impl VecExt for Vec<String> {
    fn empty_or_custom(&self, v: &Vec<String>) -> bool {
        v.is_empty()
    }
}

// --- 3. MemoryAgent 構造体 ---
struct MemoryAgent {
    #[allow(dead_code)]
    name: String,
    memory: MemoryManager,
}

impl MemoryAgent {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            memory: MemoryManager::new(4),
        }
    }

    /// ユーザー入力を受け取り、メモリを交えて応答する
    fn step(&mut self, user_input: &str) -> String {
        // 1. ユーザー入力を短期記憶に記録
        self.memory.add_short_term("User", user_input);

        // 2. メモリシステムから現在のコンテキストを構築
        let _context = self.memory.build_context(user_input);

        // 3. エージェントの推論処理（シミュレーション）
        let response = self.simulate_llm_processing(user_input);

        // 4. エージェントの応答を短期記憶に記録
        self.memory.add_short_term("Assistant", &response);

        response
    }

    /// LLMの挙動を模した内部処理
    fn simulate_llm_processing(&mut self, user_input: &str) -> String {
        // ユーザーが名前を教えてくれた場合
        if user_input.contains("私の名前は") {
            if let Some(start) = user_input.find("私の名前は") {
                let sub = &user_input[start + "私の名前は".len()..];
                if let Some(end) = sub.find("です") {
                    let name = sub[..end].trim().to_string();
                    self.memory.add_long_term("ユーザーの名前", &name);
                    return format!("承知いたしました！ あなたの名前を「{}」として記憶しました。", name);
                }
            }
        }

        // 長期記憶にアクセスする質問への対応
        if user_input.contains("私の名前を知ってる") || user_input.contains("名前は何だっけ") {
            let memories = self.memory.search_long_term("ユーザーの名前");
            if !memories.is_empty() {
                return format!("はい、覚えていますよ。{}ですね！", memories[0]);
            }
            return "まだあなたのお名前を伺っていません。".to_string();
        }

        // 通常の応答
        format!("ご入力ありがとうございます。（保持しているコンテキストに基づき処理中...）\n[受け取った入力]: {}", user_input)
    }
}

// --- 実行・動作確認テスト ---
fn main() {
    let mut agent = MemoryAgent::new("CoreAgent");

    println!("=== ターン 1 ===");
    let input1 = "こんにちは、私の名前は佐藤健一です。";
    println!("User: {}", input1);
    println!("Agent: {}\n", agent.step(input1));

    println!("=== ターン 2 ===");
    let input2 = "今日は良い天気ですね。";
    println!("User: {}", input2);
    println!("Agent: {}\n", agent.step(input2));

    println!("=== ターン 3 ===");
    let input3 = "私の名前を知っていますか？";
    println!("User: {}", input3);
    println!("Agent: {}\n", agent.step(input3));
}