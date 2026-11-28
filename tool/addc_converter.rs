// addc_converter.rs
// ScaleDeep 文脈での Address Conversion (ADDC) モジュール
// Abilene 12ノードネットワークのノード名・IPアドレス・ノードIDの相互変換
//
// 使用方法:
//   単一ファイルとして実行: rustc addc_converter.rs && ./addc_converter
//   ライブラリとして利用:   mod addc_converter;

use std::collections::HashMap;
use std::fmt;
use std::net::Ipv4Addr;
use std::str::FromStr;

// ------------------------------------------------------------
// エラー型
// ------------------------------------------------------------
#[derive(Debug, PartialEq)]
pub enum AdccError {
    UnknownNodeName(String),
    UnknownNodeId(u32),
    InvalidIpAddress(String),
    IpNotInTopology(Ipv4Addr),
    InvalidIndex(usize),
}

impl fmt::Display for AdccError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AdccError::UnknownNodeName(name) => {
                write!(f, "未知のノード名: {}", name)
            }
            AdccError::UnknownNodeId(id) => {
                write!(f, "未知のノードID: {}", id)
            }
            AdccError::InvalidIpAddress(ip) => {
                write!(f, "無効なIPアドレス文字列: {}", ip)
            }
            AdccError::IpNotInTopology(ip) => {
                write!(f, "トポロジに存在しないIP: {}", ip)
            }
            AdccError::InvalidIndex(idx) => {
                write!(f, "無効なインデックス: {}", idx)
            }
        }
    }
}

impl std::error::Error for AdccError {}

// ------------------------------------------------------------
// ノード情報構造体
// ------------------------------------------------------------
#[derive(Debug, Clone, PartialEq)]
pub struct NodeInfo {
    pub id: u32,
    pub name: String,
    pub ipv4: Ipv4Addr,
    pub city: String,
}

// ------------------------------------------------------------
// ADDC Converter
// ------------------------------------------------------------
pub struct AdccConverter {
    nodes: Vec<NodeInfo>,
    name_to_id: HashMap<String, u32>,
    ip_to_id: HashMap<Ipv4Addr, u32>,
}

impl AdccConverter {
    /// Abilene 12ノードトポロジのデフォルトコンバータを構築
    pub fn new_abilene() -> Self {
        let nodes = vec![
            NodeInfo { id: 0,  name: "ATLAM5".to_string(),  ipv4: Ipv4Addr::new(10, 0, 0, 1),  city: "Atlanta".to_string() },
            NodeInfo { id: 1,  name: "ATLAng".to_string(),  ipv4: Ipv4Addr::new(10, 0, 0, 2),  city: "Atlanta".to_string() },
            NodeInfo { id: 2,  name: "CHINng".to_string(),  ipv4: Ipv4Addr::new(10, 0, 0, 3),  city: "Chicago".to_string() },
            NodeInfo { id: 3,  name: "DNVRng".to_string(),  ipv4: Ipv4Addr::new(10, 0, 0, 4),  city: "Denver".to_string() },
            NodeInfo { id: 4,  name: "HSTNng".to_string(),  ipv4: Ipv4Addr::new(10, 0, 0, 5),  city: "Houston".to_string() },
            NodeInfo { id: 5,  name: "IPLSng".to_string(),  ipv4: Ipv4Addr::new(10, 0, 0, 6),  city: "Indianapolis".to_string() },
            NodeInfo { id: 6,  name: "KSCYng".to_string(),  ipv4: Ipv4Addr::new(10, 0, 0, 7),  city: "Kansas City".to_string() },
            NodeInfo { id: 7,  name: "LOSAng".to_string(),  ipv4: Ipv4Addr::new(10, 0, 0, 8),  city: "Los Angeles".to_string() },
            NodeInfo { id: 8,  name: "NYCMng".to_string(),  ipv4: Ipv4Addr::new(10, 0, 0, 9),  city: "New York".to_string() },
            NodeInfo { id: 9,  name: "SNVAng".to_string(),  ipv4: Ipv4Addr::new(10, 0, 0, 10), city: "Sunnyvale".to_string() },
            NodeInfo { id: 10, name: "STTLng".to_string(),  ipv4: Ipv4Addr::new(10, 0, 0, 11), city: "Seattle".to_string() },
            NodeInfo { id: 11, name: "WASHng".to_string(),  ipv4: Ipv4Addr::new(10, 0, 0, 12), city: "Washington".to_string() },
        ];

        let mut name_to_id = HashMap::new();
        let mut ip_to_id = HashMap::new();
        for node in &nodes {
            name_to_id.insert(node.name.clone(), node.id);
            ip_to_id.insert(node.ipv4, node.id);
        }

        Self { nodes, name_to_id, ip_to_id }
    }

    /// ノード名 → ノードID
    pub fn name_to_id(&self, name: &str) -> Result<u32, AdccError> {
        self.name_to_id
            .get(name)
            .copied()
            .ok_or_else(|| AdccError::UnknownNodeName(name.to_string()))
    }

    /// ノードID → ノード名
    pub fn id_to_name(&self, id: u32) -> Result<String, AdccError> {
        self.nodes
            .get(id as usize)
            .map(|n| n.name.clone())
            .ok_or(AdccError::UnknownNodeId(id))
    }

    /// ノードID → IPドレス
    pub fn id_to_ip(&self, id: u32) -> Result<Ipv4Addr, AdccError> {
        self.nodes
            .get(id as usize)
            .map(|n| n.ipv4)
            .ok_or(AdccError::UnknownNodeId(id))
    }

    /// IPアドレス → ノードID
    pub fn ip_to_id(&self, ip: Ipv4Addr) -> Result<u32, AdccError> {
        self.ip_to_id
            .get(&ip)
            .copied()
            .ok_or(AdccError::IpNotInTopology(ip))
    }

    /// IPアドレス文字列 → ノードID
    pub fn ip_str_to_id(&self, ip_str: &str) -> Result<u32, AdccError> {
        let ip = Ipv4Addr::from_str(ip_str)
            .map_err(|_| AdccError::InvalidIpAddress(ip_str.to_string()))?;
        self.ip_to_id(ip)
    }

    /// ノード名 → IPアドレス
    pub fn name_to_ip(&self, name: &str) -> Result<Ipv4Addr, AdccError> {
        let id = self.name_to_id(name)?;
        self.id_to_ip(id)
    }

    /// IPアドレス → ノード名
    pub fn ip_to_name(&self, ip: Ipv4Addr) -> Result<String, AdccError> {
        let id = self.ip_to_id(ip)?;
        self.id_to_name(id)
    }

    /// トラフィック行列の行/列インデックス → ノード名のベクトル
    pub fn index_vec_to_names(&self, indices: &[u32]) -> Result<Vec<String>, AdccError> {
        indices.iter().map(|&id| self.id_to_name(id)).collect()
    }

    /// ノード名のベクトル → トラフィック行列の行/列インデックス
    pub fn names_to_index_vec(&self, names: &[String]) -> Result<Vec<u32>, AdccError> {
        names.iter().map(|name| self.name_to_id(name)).collect()
    }

    /// 全ノード情報を取得
    pub fn all_nodes(&self) -> &[NodeInfo] {
        &self.nodes
    }

    /// ノード数
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
}

// ------------------------------------------------------------
// トラフィック行列インデックス変換ユーティリティ
// ------------------------------------------------------------
pub struct TrafficMatrixConverter<'a> {
    converter: &'a AdccConverter,
}

impl<'a> TrafficMatrixConverter<'a> {
    pub fn new(converter: &'a AdccConverter) -> Self {
        Self { converter }
    }

    /// ノード名ペア (src_name, dst_name) のリストから
    /// トラフィック行列の (src_id, dst_id) ペアリストへ変換
    pub fn flow_list_to_ids(
        &self,
        flows: &[(String, String)],
    ) -> Result<Vec<(u32, u32)>, AdccError> {
        flows
            .iter()
            .map(|(src, dst)| {
                let src_id = self.converter.name_to_id(src)?;
                let dst_id = self.converter.name_to_id(dst)?;
                Ok((src_id, dst_id))
            })
            .collect()
    }

    /// (src_id, dst_id) ペアリストから (src_name, dst_name) ペアリストへ変換
    pub fn flow_list_to_names(
        &self,
        flows: &[(u32, u32)],
    ) -> Result<Vec<(String, String)>, AdccError> {
        flows
            .iter()
            .map(|(src, dst)| {
                let src_name = self.converter.id_to_name(*src)?;
                let dst_name = self.converter.id_to_name(*dst)?;
                Ok((src_name, dst_name))
            })
            .collect()
    }
}

// ------------------------------------------------------------
// 単体テスト
// ------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name_to_id() {
        let conv = AdccConverter::new_abilene();
        assert_eq!(conv.name_to_id("CHINng").unwrap(), 2);
        assert_eq!(conv.name_to_id("WASHng").unwrap(), 11);
    }

    #[test]
    fn test_id_to_name() {
        let conv = AdccConverter::new_abilene();
        assert_eq!(conv.id_to_name(7).unwrap(), "LOSAng");
        assert_eq!(conv.id_to_name(0).unwrap(), "ATLAM5");
    }

    #[test]
    fn test_ip_conversions() {
        let conv = AdccConverter::new_abilene();
        let ip = Ipv4Addr::new(10, 0, 0, 3);
        assert_eq!(conv.ip_to_id(ip).unwrap(), 2);
        assert_eq!(conv.id_to_ip(2).unwrap(), ip);
        assert_eq!(conv.ip_to_name(ip).unwrap(), "CHINng");
        assert_eq!(conv.name_to_ip("CHINng").unwrap(), ip);
    }

    #[test]
    fn test_ip_str_to_id() {
        let conv = AdccConverter::new_abilene();
        assert_eq!(conv.ip_str_to_id("10.0.0.12").unwrap(), 11);
    }

    #[test]
    fn test_unknown_name() {
        let conv = AdccConverter::new_abilene();
        assert_eq!(
            conv.name_to_id("UNKNOWN"),
            Err(AdccError::UnknownNodeName("UNKNOWN".to_string()))
        );
    }

    #[test]
    fn test_flow_list_conversion() {
        let conv = AdccConverter::new_abilene();
        let tm_conv = TrafficMatrixConverter::new(&conv);
        let flows = vec![
            ("CHINng".to_string(), "DNVRng".to_string()),
            ("NYCMng".to_string(), "LOSAng".to_string()),
        ];
        let ids = tm_conv.flow_list_to_ids(&flows).unwrap();
        assert_eq!(ids, vec![(2, 3), (8, 7)]);

        let names_back = tm_conv.flow_list_to_names(&ids).unwrap();
        assert_eq!(names_back, flows);
    }
}

// ------------------------------------------------------------
// サンプル実行 (cargo run または rustc で単体実行時)
// ------------------------------------------------------------
fn main() {
    let conv = AdccConverter::new_abilene();

    println!("=== ADDC Converter (Abilene 12-node) ===\\n");

    // 全ノード一覧
    println!("【全ノード一覧】");
    for node in conv.all_nodes() {
        println!(
            "  ID={:2} | Name={:8} | IP={:15} | City={}",
            node.id, node.name, node.ipv4.to_string(), node.city
        );
    }

    println!("\\n【変換サンプル】");

    // ノード名 → ID
    let name = "CHINng";
    match conv.name_to_id(name) {
        Ok(id) => println!("  {} → ID {}", name, id),
        Err(e) => println!("  エラー: {}", e),
    }

    // ID → ノード名
    let id = 7u32;
    match conv.id_to_name(id) {
        Ok(name) => println!("  ID {} → {}", id, name),
        Err(e) => println!("  エラー: {}", e),
    }

    // IP → ID
    let ip = Ipv4Addr::new(10, 0, 0, 9);
    match conv.ip_to_id(ip) {
        Ok(id) => println!("  {} → ID {}", ip, id),
        Err(e) => println!("  エラー: {}", e),
    }

    // ノード名 → IP
    let name = "WASHng";
    match conv.name_to_ip(name) {
        Ok(ip) => println!("  {} → IP {}", name, ip),
        Err(e) => println!("  エラー: {}", e),
    }

    // トラフィックフローの変換
    println!("\\n【トラフィックフロー変換サンプル】");
    let tm_conv = TrafficMatrixConverter::new(&conv);
    let flows = vec![
        ("CHINng".to_string(), "DNVRng".to_string()),
        ("NYCMng".to_string(), "LOSAng".to_string()),
        ("STTLng".to_string(), "HSTNng".to_string()),
    ];
    match tm_conv.flow_list_to_ids(&flows) {
        Ok(ids) => {
            println!("  フロー(名前) → フロー(ID)");
            for ((src_name, dst_name), (src_id, dst_id)) in flows.iter().zip(ids.iter()) {
                println!("    {} → {} : ({} → {})", src_name, dst_name, src_id, dst_id);
            }
        }
        Err(e) => println!("  エラー: {}", e),
    }

    println!("\\n【エラーハンドリングサンプル】");
    match conv.name_to_id("UNKNOWN_NODE") {
        Ok(id) => println!("  ID: {}", id),
        Err(e) => println!("  予期されるエラー: {}", e),
    }

    println!("\\n=== 正常終了 ===");
}
