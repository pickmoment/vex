# harness-check

slug = `$ARGUMENTS` (없으면 사용자에게 slug 요청)

**control_dir** = `.claude/tasks/<slug>`
**state_file** = `.claude/tasks/<slug>/_state.json`

## 절차

1. `_state.json` 읽기 → `iter`, `max_iter`, `layout` 확인
2. 검증 에이전트 호출 (Explore, 읽기 전용):

```
Agent(
  subagent_type="Explore",
  prompt="""
당신은 엄격한 QA 검토자다. '충분히 좋아 보인다' 판단 금지.
모든 판정에 명령어 실행 증거를 첨부하라.

artifacts 경로: <layout.artifacts_dir>
plan.md 경로: .claude/tasks/<slug>/plan.md
verify.md 저장 경로: .claude/tasks/<slug>/verify.md

정량 기준:
- 실행 성공률 ≥ 95%
- 검증 통과율 100% (PARTIAL ≤ 2개 허용)
- 문서화 산출물: ≥ 200줄, ≥ 4 섹션

verify.md 표준 형식으로 작성:
\`\`\`markdown
## 검증 결과: PASS | FAIL | PARTIAL

| 항목 | 기준 | 실측값 | 판정 | 증거 |
|------|------|--------|------|------|

### 실패 원인 및 수정 지시사항
- [ ] 수정 항목
\`\`\`

보고 형식:
[판정] PASS | FAIL | PARTIAL(<n>개 미충족)
[요약] 한 줄 요약
"""
)
```

3. 판정 처리 후 `_state.json` 갱신:
   - **PASS** 또는 **PARTIAL(≤2개)**: `phase: "done"`
   - **FAIL** 또는 **PARTIAL(≥3개)**:
     - `iter + 1 < max_iter` → `iter++`, `phase: "plan"`
     - `iter + 1 >= max_iter` → `status: "exhausted"`
4. 판정과 다음 단계를 사용자에게 보고
