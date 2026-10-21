export interface FlashcardGenerateResult {
  success: boolean;
  message: string;
  cards_generated: number;
  audio_clips: number;
  snapshots: number;
  video_clips: number;
  tsv_path: string | null;
  apkg_path: string | null;
  output_size_bytes?: number;
}
