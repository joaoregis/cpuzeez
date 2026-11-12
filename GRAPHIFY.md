# Graphify neste projeto

Grafo de conhecimento do código em `graphify-out/` (nós, comunidades, relações entre arquivos).
Consultar o grafo custa bem menos token do que varrer arquivos com grep/Read.

## Setup (uma vez por projeto)

```bash
/graphify .                # constrói o grafo (na sessão do Claude Code)
graphify claude install    # seção no CLAUDE.md + hook PreToolUse: o Claude consulta o grafo antes de varrer arquivos
graphify hook install      # git hooks (post-commit/checkout/merge/rewrite) que atualizam o grafo sozinhos, sem LLM
```

`graphify claude install` altera o CLAUDE.md e as settings do projeto. Remover: `graphify claude uninstall` e `graphify hook uninstall`.

## Uso no dia a dia

| Comando | Para quê |
|---|---|
| `graphify query "pergunta"` | Contexto amplo (BFS). `--dfs` segue um caminho específico, `--budget N` limita tokens |
| `graphify path "A" "B"` | Menor caminho / relação entre dois conceitos |
| `graphify explain "X"` | Explicação de um nó e seus vizinhos |
| `graphify update .` | Reextrai só arquivos de código novos/alterados (AST, sem custo de API). Rodar após alterar código |
| `/graphify . --update` | Idem, incluindo docs/imagens (usa LLM só no que mudou) |
| `/graphify . --mode deep` | Extração mais rica e mais cara. Só em projeto muito explorado |
| `/graphify . --mcp` | Servidor MCP pro agente consultar o grafo como ferramenta |
| `/graphify . --wiki` | Wiki navegável (`graphify-out/wiki/index.md`) |

Ordem de consulta: `query` / `path` / `explain` primeiro; `GRAPH_REPORT.md` só para visão geral de arquitetura; arquivo-fonte só para as linhas que o grafo já apontou.

## `.graphifyignore`

Excluir o que não é código-fonte relevante reduz custo e ruído. Sugestão de partida:

```
node_modules/
dist/
build/
.venv/
__pycache__/
*.min.js
*.lock
```

Ajustar conforme a stack do projeto.

## Notas

- `graphify-out/graph.json` é reescrito inteiro a cada build: diff grande nele é esperado.
- Código é extraído por AST (grátis). Docs, imagens e vídeos usam subagentes do Claude (gastam token); em repo grande, comece por uma pasta.
- Regras para o agente estão no `E:\Projects\CLAUDE.md` (vale para todos os projetos sob `E:\Projects`).
