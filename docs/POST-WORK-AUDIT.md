# 문서 정리 체크리스트

> 구현 작업 완료 후, 아래 항목을 순서대로 점검한다.

---

## 1. active → completed 이동

- [ ] `exec-plans/active/` 내 모든 파일을 열어 **구현 완료 여부** 확인
  - 플랜에 명시된 파일이 실제 코드에 존재하는가?
  - 테스트가 모두 구현되었는가?
- [ ] 완료된 플랜에 **Completion 섹션** 작성 (날짜, 요약, 변경 파일, 검증, 후속 작업)
- [ ] `git mv docs/exec-plans/active/{plan}.md docs/exec-plans/completed/{plan}.md`

## 2. INDEX.md 동기화

- [ ] `exec-plans/active/`, `exec-plans/completed/` 실제 파일 목록과 INDEX.md 항목 일치 확인
- [ ] 누락된 파일 → INDEX.md에 추가
- [ ] 삭제된 파일 → INDEX.md에서 제거
- [ ] 상태값(`active` / `complete`) 정확한지 확인

## 3. STATUS.md 동기화

- [ ] INDEX.md의 exec-plans 항목과 STATUS.md 구현 계획 표가 일치하는지 확인
- [ ] 상태 아이콘 및 경로(`active/` vs `completed/`) 정확한지 확인
- [ ] 헤더 날짜를 오늘 날짜로 갱신

## 4. 디렉토리 구조

- [ ] INDEX.md Structure에 명시된 카테고리 디렉토리가 실제 존재하는지 확인
- [ ] 빈 디렉토리는 `.gitkeep` 유지

## 5. 링크 검증

- [ ] INDEX.md 내 상대 링크가 깨지지 않았는지 확인
- [ ] STATUS.md 내 상대 링크가 깨지지 않았는지 확인
- [ ] completed/ 이동 시 exec-plan 내부의 상대 링크 경로도 수정했는지 확인
