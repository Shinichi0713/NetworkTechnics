mod storage;
mod task;

use std::env;
use task::TaskList;
use storage::Storage;

fn print_help() {
    println!("Rust Task Manager - 使い方");
    println!();
    println!("  cargo run -- add <title> [description]     タスクを追加");
    println!("  cargo run -- list                          すてのタスクを表示");
    println!("  cargo run -- done <id>                     タスクを完了/未完了に切り替え");
    println!("  cargo run -- delete <id>                   タスクを削除");
    println!("  cargo run -- update <id> <title> [desc]    タスクを更新");
    println!("  cargo run -- completed                     完了済みタスクを表示");
    println!("  cargo run -- pending                       未完了タスクを表示");
    println!("  cargo run -- help                          このヘルプを表示");
}

fn print_task(task: &task::Task) {
    let status = if task.completed { "✅ 完了" } else { "⬜ 未完了" };
    println!(
        "[{}] {} - {} ({})",
        task.id, status, task.title, task.description
    );
    println!("    作成: {} | 更新: {}",
        task.created_at.format("%Y-%m-%d %H:%M"),
        task.updated_at.format("%Y-%m-%d %H:%M")
    );
}

fn print_tasks(tasks: &[&task::Task]) {
    if tasks.is_empty() {
        println!("タスクがありません。");
        return;
    }
    for task in tasks {
        print_task(task);
        println!();
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_help();
        return;
    }

    let command = args[1].as_str();
    let mut tasks = Storage::load(None).unwrap_or_else(|e| {
        eprintln!("ファイル読み込みエラー: {}", e);
        TaskList::new()
    });

    match command {
        "add" => {
            if args.len() < 3 {
                println!("使い方: cargo run -- add <title> [description]");
                return;
            }
            let title = &args[2];
            let description = if args.len() > 3 {
                args[3..].join(" ")
            } else {
                String::new()
            };
            let task = tasks.add(title, &description);
            println!("タスクを追加しました:");
            print_task(task);
        }

        "list" => {
            if tasks.is_empty() {
                println!("タスクがありません。");
            } else {
                println!("=== すべてのタスク ===");
                for task in tasks.list_all() {
                    print_task(task);
                    println!();
                }
            }
        }

        "done" | "toggle" => {
            if args.len() < 3 {
                println!("使い方: cargo run -- done <id>");
                return;
            }
            let id: usize = match args[2].parse() {
                Ok(n) => n,
                Err(_) => {
                    println!("IDは数字で指定してください。");
                    return;
                }
            };
            match tasks.toggle(id) {
                Some(completed) => {
                    let status = if completed { "完了" } else { "未完了" };
                    println!("タスク [{}] を{}にしました。", id, status);
                    if let Some(task) = tasks.get(id) {
                        print_task(task);
                    }
                }
                None => println!("タスク ID:{} が見つかりません。", id),
            }
        }

        "delete" | "remove" => {
            if args.len() < 3 {
                println!("使い方: cargo run -- delete <id>");
                return;
            }
            let id: usize = match args[2].parse() {
                Ok(n) => n,
                Err(_) => {
                    println!("IDは数字で指定してください。");
                    return;
                }
            };
            match tasks.remove(id) {
                Some(task) => {
                    println!("タスクを削除しました:");
                    print_task(&task);
                }
                None => println!("タスク ID:{} が見つかりません。", id),
            }
        }

        "update" => {
            if args.len() < 4 {
                println!("使い方: cargo run -- update <id> <title> [description]");
                return;
            }
            let id: usize = match args[2].parse() {
                Ok(n) => n,
                Err(_) => {
                    println!("IDは数字で指定してください。");
                    return;
                }
            };
            let title = &args[3];
            let description = if args.len() > 4 {
                args[4..].join(" ")
            } else {
                String::new()
            };
            match tasks.get_mut(id) {
                Some(task) => {
                    task.update(title, &description);
                    println!("タスクを更新しました:");
                    print_task(task);
                }
                None => println!("タスク ID:{} が見つかりません。", id),
            }
        }

        "completed" => {
            let completed = tasks.list_completed();
            if completed.is_empty() {
                println!("完了済みのタスクがありません。");
            } else {
                println!("=== 完了済みタスク ===");
                print_tasks(&completed);
            }
        }

        "pending" => {
            let pending = tasks.list_pending();
            if pending.is_empty() {
                println!("未完了のタスクがありません。");
            } else {
                println!("=== 未完了タスク ===");
                print_tasks(&pending);
            }
        }

        "help" | "--help" | "-h" => {
            print_help();
        }

        _ => {
            println!("不明なコマンド: {}", command);
            print_help();
        }
    }

    if let Err(e) = Storage::save(&tasks, None) {
        eprintln!("保存エラー: {}", e);
    }
}
