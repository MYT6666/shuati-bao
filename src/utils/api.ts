import { invoke } from '@tauri-apps/api/core'

export interface QuizBank {
  id: number; name: string; description: string | null;
  question_count: number; created_at: string; updated_at: string;
}
export interface Question {
  id: number; bank_id: number; type: string; stem: string;
  options: string | null; answer: string | null; analysis: string | null;
  source_index: number | null; confidence: number;
}
export interface NewBank { name: string; description: string | null }

export const api = {
  listBanks: () => invoke<QuizBank[]>('list_banks'),
  createBank: (b: NewBank) => invoke<QuizBank>('create_bank', { newBank: b }),
  deleteBank: (id: number) => invoke<void>('delete_bank', { bankId: id }),
  listQuestions: (bankId: number) => invoke<Question[]>('list_questions', { bankId }),
  clearBankQuestions: (bankId: number) => invoke<void>('clear_bank_questions', { bankId }),
  updateQuestion: (q: Question) => invoke<void>('update_question', { q }),
  searchQuestions: (bankId: number, query: string, limit?: number) => invoke<Question[]>('search_questions', { bankId, query, limit }),
  analyzeQuestion: (q: Question) => invoke<string>('analyze_question', { q }),
  importFromHtml: (bankId: number, html: string) => invoke<number>('import_from_html', { bankId, html }),
  importWithAi: (bankId: number, text: string) => invoke<{ count: number; expected: number }>('import_with_ai', { bankId, text }),
  testAiConnection: () => invoke<void>('test_ai_connection'),
  cancelImport: () => invoke<void>('cancel_import'),
  recordPractice: (r: { bank_id: number; question_id: number; user_answer: string | null; is_correct: boolean; duration_ms: number | null }) =>
    invoke<void>('record_practice', { record: { ...r, practiced_at: new Date().toISOString() } }),
  listWrong: (bankId: number) => invoke<number[]>('list_wrong', { bankId }),
  listMastered: (bankId: number) => invoke<number[]>('list_mastered', { bankId }),
  markWrongMastered: (bankId: number, questionId: number) => invoke<void>('mark_wrong_mastered', { bankId, questionId }),
  bankStats: (bankId: number) => invoke<{ total: number; practiced: number; correct: number }>('bank_stats', { bankId }),
  getSetting: (key: string) => invoke<string | null>('get_setting', { key }),
  setSetting: (key: string, value: string) => invoke<void>('set_setting', { key, value }),
  ocrAvailable: () => invoke<boolean>('ocr_available'),
  toggleFavorite: (bankId: number, questionId: number) => invoke<boolean>('toggle_favorite', { bankId, questionId }),
  listFavorites: (bankId: number) => invoke<number[]>('list_favorites', { bankId }),
  isFavorite: (bankId: number, questionId: number) => invoke<boolean>('is_favorite', { bankId, questionId }),
  clearFavorites: (bankId: number) => invoke<void>('clear_favorites', { bankId }),
  backupDatabase: () => invoke<string>('backup_database'),
  exportBank: (bankId: number) => invoke<string>('export_bank', { bankId }),
}
