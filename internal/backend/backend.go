package backend

import "context"

// IterationOptions controls how a backend executes a single iteration.
type IterationOptions struct {
	Prompt        string
	Model         string
	OutputFile    string
	RawOutputFile string
}

// Backend defines the interface for AI backends.
type Backend interface {
	CheckInstalled() error
	GetModels() []string
	RunIteration(ctx context.Context, opts IterationOptions) error
	ParseText(path string) (string, error)
}
