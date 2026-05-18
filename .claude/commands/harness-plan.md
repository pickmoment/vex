# harness-plan

slug = `$ARGUMENTS` (없으면 사용자에게 slug 요청)

**control_dir** = `.claude/tasks/<slug>`
**state_file** = `.claude/tasks/<slug>/_state.json`

## 절차

1. `_state.json` 읽기 → `iter`, `layout` 확인
2. Opus 플래너 에이전트 호출 (경로만 전달):

```
Agent(
  model="opus",
  subagent_type="Plan",
  prompt="""
아래 파일을 직접 읽어 실행 계획을 수립하라.

PRD.md 경로: PRD.md
plan.md 저장 경로: .claude/tasks/<slug>/plan.md
현재 iter: <iter>
(iter > 0) verify.md 경로: .claude/tasks/<slug>/verify.md — 실패 원인 반영

각 스테이지 필수 포함: 제목·순차/병렬·입력·출력·도구·성공기준.
하단에 병렬 실행 가능 스테이지 쌍 명시.

보고 형식:
[결과] 성공 | 실패
[저장 경로] .claude/tasks/<slug>/plan.md
[스테이지] <구조 한 줄, 예: S1→[S2a‖S2b]→S3>
[요약] 한 줄
[실패 사유] (실패 시)
"""
)
```

3. 보고 수신 후 즉시: `_state.json` → `phase: "do"` 갱신
4. 결과를 사용자에게 보고
