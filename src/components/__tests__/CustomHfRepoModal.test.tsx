import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import '../../i18n';
import { CustomHfRepoModal } from '../CustomHfRepoModal';

describe('CustomHfRepoModal', () => {
  it('renders nothing when open=false', () => {
    render(<CustomHfRepoModal open={false} onClose={vi.fn()} onSave={vi.fn()} />);
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  it('renders title, three fields, save+cancel when open', () => {
    render(<CustomHfRepoModal open={true} onClose={vi.fn()} onSave={vi.fn()} />);
    expect(screen.getByRole('dialog')).toBeInTheDocument();
    expect(screen.getByLabelText(/hugging face repo/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/gguf filename/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/mmproj filename/i)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /add model/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /cancel/i })).toBeInTheDocument();
  });

  it('disables save until hf_repo + gguf_filename are valid', async () => {
    const user = userEvent.setup();
    render(<CustomHfRepoModal open={true} onClose={vi.fn()} onSave={vi.fn()} />);
    const save = screen.getByRole('button', { name: /add model/i });
    expect(save).toBeDisabled();

    await user.type(screen.getByLabelText(/hugging face repo/i), 'owner/repo');
    expect(save).toBeDisabled();

    await user.type(screen.getByLabelText(/gguf filename/i), 'model.gguf');
    expect(save).toBeEnabled();
  });

  it('shows repo-format error when hf_repo does not match owner/repo', async () => {
    const user = userEvent.setup();
    render(<CustomHfRepoModal open={true} onClose={vi.fn()} onSave={vi.fn()} />);
    await user.type(screen.getByLabelText(/hugging face repo/i), 'just-owner');
    expect(screen.getByText(/must be owner\/repo/i)).toBeInTheDocument();
  });

  it('shows gguf-extension error when gguf_filename lacks .gguf suffix', async () => {
    const user = userEvent.setup();
    render(<CustomHfRepoModal open={true} onClose={vi.fn()} onSave={vi.fn()} />);
    await user.type(screen.getByLabelText(/gguf filename/i), 'model.bin');
    expect(screen.getByText(/must end with \.gguf/i)).toBeInTheDocument();
  });

  it('shows mmproj-same error when mmproj_filename equals gguf_filename', async () => {
    const user = userEvent.setup();
    render(<CustomHfRepoModal open={true} onClose={vi.fn()} onSave={vi.fn()} />);
    await user.type(screen.getByLabelText(/hugging face repo/i), 'owner/repo');
    await user.type(screen.getByLabelText(/gguf filename/i), 'model.gguf');
    await user.type(screen.getByLabelText(/mmproj filename/i), 'model.gguf');
    expect(screen.getByText(/must differ from main gguf/i)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /add model/i })).toBeDisabled();
  });

  it('omits mmproj_filename from payload when field is empty', async () => {
    const user = userEvent.setup();
    const onSave = vi.fn();
    render(<CustomHfRepoModal open={true} onClose={vi.fn()} onSave={onSave} />);
    await user.type(screen.getByLabelText(/hugging face repo/i), 'TheBloke/Llama-2-7B-Chat-GGUF');
    await user.type(screen.getByLabelText(/gguf filename/i), 'llama-2-7b-chat.Q4_K_M.gguf');
    await user.click(screen.getByRole('button', { name: /add model/i }));
    expect(onSave).toHaveBeenCalledWith({
      hf_repo: 'TheBloke/Llama-2-7B-Chat-GGUF',
      gguf_filename: 'llama-2-7b-chat.Q4_K_M.gguf',
    });
  });

  it('includes mmproj_filename in payload when provided', async () => {
    const user = userEvent.setup();
    const onSave = vi.fn();
    render(<CustomHfRepoModal open={true} onClose={vi.fn()} onSave={onSave} />);
    await user.type(
      screen.getByLabelText(/hugging face repo/i),
      'Qwen/Qwen2.5-VL-7B-Instruct-GGUF',
    );
    await user.type(screen.getByLabelText(/gguf filename/i), 'qwen2.5-vl-7b-instruct-q4_k_m.gguf');
    await user.type(screen.getByLabelText(/mmproj filename/i), 'mmproj-qwen2.5-vl-7b-f16.gguf');
    await user.click(screen.getByRole('button', { name: /add model/i }));
    expect(onSave).toHaveBeenCalledWith({
      hf_repo: 'Qwen/Qwen2.5-VL-7B-Instruct-GGUF',
      gguf_filename: 'qwen2.5-vl-7b-instruct-q4_k_m.gguf',
      mmproj_filename: 'mmproj-qwen2.5-vl-7b-f16.gguf',
    });
  });

  it('invokes onClose on Cancel click', async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    render(<CustomHfRepoModal open={true} onClose={onClose} onSave={vi.fn()} />);
    await user.click(screen.getByRole('button', { name: /cancel/i }));
    expect(onClose).toHaveBeenCalled();
  });

  it('invokes onClose on Escape', async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    render(<CustomHfRepoModal open={true} onClose={onClose} onSave={vi.fn()} />);
    await user.keyboard('{Escape}');
    expect(onClose).toHaveBeenCalled();
  });

  it('does not call onSave on Add when validation fails', async () => {
    const user = userEvent.setup();
    const onSave = vi.fn();
    render(<CustomHfRepoModal open={true} onClose={vi.fn()} onSave={onSave} />);
    await user.type(screen.getByLabelText(/hugging face repo/i), 'just-owner');
    await user.type(screen.getByLabelText(/gguf filename/i), 'model.gguf');
    const save = screen.getByRole('button', { name: /add model/i });
    expect(save).toBeDisabled();
    await user.click(save);
    expect(onSave).not.toHaveBeenCalled();
  });
});
