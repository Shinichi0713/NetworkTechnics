承知いたしました。クイズではなく、実際にコードを書いてエラー処理の勘所を掴むための**実装演習問題**を出題します。

今回のテーマは、**「ユーザー登録システムのバリデーション」**です。
`Result` 型を使って、不正な入力があった場合に適切なエラーを返す仕組みを作ってみましょう。

---

### 💻 実装課題：ユーザー登録バリデーター

以下の仕様と雛形コードに従って、プログラムを完成させてください。

#### 1. エラー型の定義

`RegistrationError` という名前の `enum` を定義してください。

* `EmptyName`: 名前が空（""）の場合。
* `InvalidAge`: 年齢が 0 未満、または 150 歳以上の場合。

#### 2. バリデーション関数の実装

`register_user` という関数を実装してください。

* **引数**: `name: &str`, `age: i32`
* **戻り値**: `Result<(), RegistrationError>`
* **処理内容**:
* 名前が空なら `RegistrationError::EmptyName` を `Err` で返す。
* 年齢が範囲外なら `RegistrationError::InvalidAge` を `Err` で返す。
* 両方問題なければ `Ok(())` を返す。



#### 3. main関数でのエラーハンドリング

`main` 関数の中で `register_user` を呼び出し、**`match` 式** を使って以下の処理を行ってください。

* 成功した場合：「登録成功！」と表示。
* 名前が空エラーの場合：「名前を入力してください」と表示。
* 年齢エラーの場合：「正しい年齢を入力してください」と表示。

---

### 📝 雛形コード

```rust
// 1. エラー型の定義 (Debugをderiveしておくと便利です)
#[derive(Debug)]
enum RegistrationError {
    todo!("ここに列挙子を追加してください")
}

// 2. バリデーション関数の実装
fn register_user(name: &str, age: i32) -> Result<(), RegistrationError> {
    // ここに条件分岐を実装してください
    todo!()
}

fn main() {
    let test_name = "";
    let test_age = 25;

    // 3. match式によるエラーハンドリング
    match register_user(test_name, test_age) {
        todo!("各ケース（Ok, Err）に応じた処理を書いてください")
    }
}

```

---

### 💡 ヒント

* **Resultの返し方**: 成功は `Ok(())`、失敗は `Err(RegistrationError::...)` と書きます。
* **文字列の判定**: `name.is_empty()` メソッドを使うと便利です。
* **matchの書き方**:
```rust
match result {
    Ok(_) => { /* 成功 */ },
    Err(RegistrationError::EmptyName) => { /* 名前エラー */ },
    // ...
}

```



実装が完了したら、コードを教えてください！正しくエラーを「値」として扱えているか確認させていただきます。

**これができたら、次は「`?` 演算子を使って複数のバリデーションを連結する方法」に挑戦してみますか？**