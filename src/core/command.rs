/// Undo/Redo を支えるコマンドパターン。
/// 全ての編集操作はこのトレイトを実装する。
use anyhow::Result;
use std::fmt;

/// 実行可能かつ取り消し可能な操作を表すトレイト。
pub trait Command<Ctx>: fmt::Debug {
    /// コマンドを実行し、状態を変更する。
    fn execute(&mut self, ctx: &mut Ctx) -> Result<()>;
    /// コマンドを取り消し、状態を元に戻す。
    fn undo(&mut self, ctx: &mut Ctx) -> Result<()>;
    /// UI表示用の操作名。
    fn description(&self) -> &str;
}

/// コマンド履歴を管理するスタック。Undo/Redo を提供する。
pub struct CommandStack<Ctx> {
    /// 実行済みコマンド (Undo用)
    pub undo_stack: Vec<Box<dyn Command<Ctx>>>,
    /// Undo されたコマンド (Redo用)
    pub redo_stack: Vec<Box<dyn Command<Ctx>>>,
    /// 履歴の最大保持数 (メモリ制限)
    max_history: usize,
}

impl<Ctx> CommandStack<Ctx> {
    /// 新しい CommandStack を作成する。
    /// `max_history`: 保持する最大コマンド数。
    pub fn new(max_history: usize) -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_history,
        }
    }

    /// コマンドを実行し、履歴に追加する。
    /// 新しいコマンドが実行されると Redo 履歴はクリアされる。
    pub fn execute(&mut self, mut cmd: Box<dyn Command<Ctx>>, ctx: &mut Ctx) -> Result<()> {
        cmd.execute(ctx)?;
        log::debug!("コマンド実行: {:?}", cmd.description());
        self.undo_stack.push(cmd);
        self.redo_stack.clear();

        // メモリ制限: 古いものから削除
        while self.undo_stack.len() > self.max_history {
            self.undo_stack.remove(0);
        }
        Ok(())
    }

    /// 最後のコマンドを取り消す。
    pub fn undo(&mut self, ctx: &mut Ctx) -> Result<bool> {
        if let Some(mut cmd) = self.undo_stack.pop() {
            log::debug!("Undo: {:?}", cmd.description());
            cmd.undo(ctx)?;
            self.redo_stack.push(cmd);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// 取り消されたコマンドを再実行する。
    pub fn redo(&mut self, ctx: &mut Ctx) -> Result<bool> {
        if let Some(mut cmd) = self.redo_stack.pop() {
            log::debug!("Redo: {:?}", cmd.description());
            cmd.execute(ctx)?;
            self.undo_stack.push(cmd);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Undo 可能かどうか。
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Redo 可能かどうか。
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// 履歴をすべてクリアする。
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    /// 外部で実行されたコマンドを履歴に追加する (ブラシ等で使用)。
    #[allow(dead_code)]
    pub fn push_to_undo(&mut self, cmd: Box<dyn Command<Ctx>>) {
        self.undo_stack.push(cmd);
        self.redo_stack.clear();
        while self.undo_stack.len() > self.max_history {
            self.undo_stack.remove(0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct TestCommand {
        value: i32,
        executed: bool,
    }

    impl Command<i32> for TestCommand {
        fn execute(&mut self, ctx: &mut i32) -> Result<()> {
            self.executed = true;
            *ctx = self.value;
            Ok(())
        }
        fn undo(&mut self, _ctx: &mut i32) -> Result<()> {
            self.executed = false;
            Ok(())
        }
        fn description(&self) -> &str {
            "Test Command"
        }
    }

    #[test]
    fn test_undo_redo_cycle() {
        let mut stack = CommandStack::<i32>::new(100);
        let mut ctx = 0i32;

        let cmd = Box::new(TestCommand {
            value: 42,
            executed: false,
        });
        stack.execute(cmd, &mut ctx).unwrap();
        assert_eq!(ctx, 42);

        assert!(stack.can_undo());
        assert!(!stack.can_redo());

        stack.undo(&mut ctx).unwrap();
        assert!(!stack.can_undo());
        assert!(stack.can_redo());

        stack.redo(&mut ctx).unwrap();
        assert!(stack.can_undo());
        assert!(!stack.can_redo());
        assert_eq!(ctx, 42);
    }

    #[test]
    fn test_max_history_limit() {
        let mut stack = CommandStack::<i32>::new(3);
        let mut ctx = 0i32;

        for i in 0..5 {
            let cmd = Box::new(TestCommand {
                value: i,
                executed: false,
            });
            stack.execute(cmd, &mut ctx).unwrap();
        }

        // max_history=3 なので最大3つしか残らない
        assert_eq!(stack.undo_stack.len(), 3);
    }
}
