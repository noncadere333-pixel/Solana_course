# Область `vault` — Программа `liquidity-vault` и клиент `vault-client`

## 1. Структура отдельного Workspace

Проект `liquidity-vault` выделен в самостоятельный Cargo/Anchor workspace:
- `programs/liquidity-vault` — ончейн-программа на Anchor `1.2.0` (`anchor-spl = 1.2.2`).
- `clients/vault-client` — CLI-клиент на Rust без внешних TypeScript SDK для управления хранилищем, пулом Raydium CLMM, стратегией, обменами и концентрированной позицией.
- `app/` — веб-интерфейс (Next.js 16) для недели 14 (Capstone Build Week): панель вкладчика и блок действий администратора на одной странице.

## 2. Сборка SBF и локальный валидатор / тесты

1. Сборка SBF-бинарника программы:
   ```bash
   cargo build-sbf --manifest-path programs/liquidity-vault/Cargo.toml
   ```
   Артефакт создаётся в `target/deploy/liquidity_vault.so`.
2. Запуск интеграционных тестов LiteSVM:
   ```bash
   cargo test -p liquidity-vault --lib     # модульные тесты формул
   cargo test -p liquidity-vault --test vault
   ```

## 3. Архитектура PDA и владение средствами

- `VaultState` (`seeds = [b"vault", deposit_mint.as_ref()]`) — центральный PDA хранилища, который выступает:
  - `mint::authority` для `share_mint` (`seeds = [b"share_mint", vault.key().as_ref()]`);
  - `token::authority` для основного хранилища `token_vault` (`seeds = [b"vault_token", vault.key().as_ref()]`);
- `UserCard` (`seeds = [b"user_card", vault.key().as_ref(), user.key().as_ref()]`) — персональная карточка вкладчика.
