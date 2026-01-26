package server

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"net/http"
	"net/url"
	"os"
	"os/exec"
	"strconv"
	"strings"
	"syscall"
	"time"

	"github.com/goosewin/gralph/internal/core"
	"github.com/goosewin/gralph/internal/state"
)

const (
	defaultHost         = "127.0.0.1"
	defaultPort         = 8080
	defaultMaxBodyBytes = 4096
)

// Options configures the HTTP status server.
type Options struct {
	Host         string
	Port         int
	Token        string
	Open         bool
	MaxBodyBytes int64
}

// StartServer runs the HTTP status server until ctx is canceled.
func StartServer(ctx context.Context, opts Options) error {
	host := strings.TrimSpace(opts.Host)
	if host == "" {
		host = defaultHost
	}
	port := opts.Port
	if port == 0 {
		port = defaultPort
	}
	if port < 1 || port > 65535 {
		return fmt.Errorf("invalid port number: %d", port)
	}
	maxBody := opts.MaxBodyBytes
	if maxBody <= 0 {
		maxBody = defaultMaxBodyBytes
	}

	if err := state.InitState(); err != nil {
		return err
	}

	srv := &http.Server{
		Addr:              fmt.Sprintf("%s:%d", host, port),
		ReadHeaderTimeout: 5 * time.Second,
		IdleTimeout:       30 * time.Second,
		Handler:           newHandler(handlerOptions{host: host, token: opts.Token, open: opts.Open, maxBody: maxBody}),
	}

	shutdownErr := make(chan error, 1)
	go func() {
		<-ctx.Done()
		ctxTimeout, cancel := context.WithTimeout(context.Background(), 5*time.Second)
		defer cancel()
		shutdownErr <- srv.Shutdown(ctxTimeout)
	}()

	err := srv.ListenAndServe()
	if errors.Is(err, http.ErrServerClosed) {
		select {
		case shutdownErr := <-shutdownErr:
			return shutdownErr
		default:
			return nil
		}
	}
	return err
}

type handlerOptions struct {
	host    string
	token   string
	open    bool
	maxBody int64
}

func newHandler(opts handlerOptions) http.Handler {
	mux := http.NewServeMux()
	mux.HandleFunc("/status", func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/status" {
			writeJSONError(w, http.StatusNotFound, "Unknown endpoint")
			return
		}
		if !authorizeRequest(w, r, opts) {
			return
		}
		if r.Method != http.MethodGet {
			writeJSONError(w, http.StatusMethodNotAllowed, "Method not allowed")
			return
		}
		response := listSessionsResponse()
		writeJSON(w, http.StatusOK, response)
	})

	mux.HandleFunc("/status/", func(w http.ResponseWriter, r *http.Request) {
		if !authorizeRequest(w, r, opts) {
			return
		}
		if r.Method != http.MethodGet {
			writeJSONError(w, http.StatusMethodNotAllowed, "Method not allowed")
			return
		}
		name, ok := pathRemainder(r.URL.Path, "/status/")
		if !ok {
			writeJSONError(w, http.StatusNotFound, "Unknown endpoint")
			return
		}
		session, found, err := getSessionResponse(name)
		if err != nil {
			writeJSONError(w, http.StatusInternalServerError, "Failed to read session")
			return
		}
		if !found {
			writeJSONError(w, http.StatusNotFound, fmt.Sprintf("Session not found: %s", name))
			return
		}
		writeJSON(w, http.StatusOK, session)
	})

	mux.HandleFunc("/stop/", func(w http.ResponseWriter, r *http.Request) {
		if !authorizeRequest(w, r, opts) {
			return
		}
		if r.Method != http.MethodPost {
			writeJSONError(w, http.StatusMethodNotAllowed, "Method not allowed")
			return
		}
		name, ok := pathRemainder(r.URL.Path, "/stop/")
		if !ok {
			writeJSONError(w, http.StatusNotFound, "Unknown endpoint")
			return
		}
		if err := stopSession(name); err != nil {
			if errors.Is(err, errSessionNotFound) {
				writeJSONError(w, http.StatusNotFound, fmt.Sprintf("Session not found: %s", name))
				return
			}
			writeJSONError(w, http.StatusInternalServerError, "Failed to stop session")
			return
		}
		writeJSON(w, http.StatusOK, map[string]interface{}{"success": true, "message": "Session stopped"})
	})

	mux.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/" {
			writeJSONError(w, http.StatusNotFound, "Unknown endpoint")
			return
		}
		if !authorizeRequest(w, r, opts) {
			return
		}
		if r.Method != http.MethodGet {
			writeJSONError(w, http.StatusMethodNotAllowed, "Method not allowed")
			return
		}
		writeJSON(w, http.StatusOK, map[string]string{"status": "ok", "service": "gralph-server"})
	})

	return withCORS(mux, opts)
}

func withCORS(next http.Handler, opts handlerOptions) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		corsOrigin := resolveCORSOrigin(r.Header.Get("Origin"), opts.host, opts.open)
		if corsOrigin != "" {
			w.Header().Set("Access-Control-Allow-Origin", corsOrigin)
			if corsOrigin != "*" {
				w.Header().Set("Vary", "Origin")
			}
			w.Header().Set("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
			w.Header().Set("Access-Control-Allow-Headers", "Authorization, Content-Type")
			w.Header().Set("Access-Control-Expose-Headers", "Content-Length, Content-Type")
			w.Header().Set("Access-Control-Max-Age", "86400")
		}

		if r.Method == http.MethodOptions {
			w.WriteHeader(http.StatusNoContent)
			return
		}

		if opts.maxBody > 0 {
			r.Body = http.MaxBytesReader(w, r.Body, opts.maxBody)
		}

		next.ServeHTTP(w, r)
	})
}

func authorizeRequest(w http.ResponseWriter, r *http.Request, opts handlerOptions) bool {
	if opts.token == "" {
		return true
	}
	header := strings.TrimSpace(r.Header.Get("Authorization"))
	fields := strings.Fields(header)
	if len(fields) != 2 || !strings.EqualFold(fields[0], "Bearer") || fields[1] != opts.token {
		writeJSONError(w, http.StatusUnauthorized, "Invalid or missing Bearer token")
		return false
	}
	return true
}

func resolveCORSOrigin(origin, host string, open bool) string {
	origin = strings.TrimSpace(origin)
	if origin == "" {
		return ""
	}
	if open {
		return "*"
	}

	switch origin {
	case "http://localhost", "http://127.0.0.1", "http://[::1]":
		return origin
	}

	host = strings.TrimSpace(host)
	if host != "" && host != "0.0.0.0" && host != "::" {
		if origin == "http://"+host {
			return origin
		}
	}
	return ""
}

func pathRemainder(path, prefix string) (string, bool) {
	if !strings.HasPrefix(path, prefix) {
		return "", false
	}
	remainder := strings.TrimPrefix(path, prefix)
	if remainder == "" {
		return "", false
	}
	decoded, err := url.PathUnescape(remainder)
	if err != nil {
		return "", false
	}
	return decoded, true
}

type listResponse struct {
	Sessions []map[string]interface{} `json:"sessions"`
}

func listSessionsResponse() listResponse {
	sessions, err := state.ListSessions()
	if err != nil {
		return listResponse{Sessions: []map[string]interface{}{}}
	}
	response := listResponse{Sessions: make([]map[string]interface{}, 0, len(sessions))}
	for _, session := range sessions {
		response.Sessions = append(response.Sessions, enrichSession(session))
	}
	return response
}

func getSessionResponse(name string) (map[string]interface{}, bool, error) {
	session, found, err := state.GetSession(name)
	if err != nil {
		return nil, false, err
	}
	if !found {
		return nil, false, nil
	}
	return enrichSession(session), true, nil
}

func enrichSession(session state.Session) map[string]interface{} {
	fields := copyFields(session.Fields)
	fields = ensureNameField(session.Name, fields)
	dir := stringField(fields, "dir")
	taskFile := stringField(fields, "task_file")
	if taskFile == "" {
		taskFile = "PRD.md"
	}

	remaining := -1
	if dir != "" && taskFile != "" {
		path := dir + "/" + taskFile
		if count, err := core.CountRemainingTasks(path); err == nil {
			remaining = count
		}
	}
	if remaining < 0 {
		if count, ok := intField(fields, "last_task_count"); ok {
			remaining = count
		}
	}

	status := stringField(fields, "status")
	pid, _ := intField(fields, "pid")
	isAlive := false
	if status == "running" && pid > 0 {
		if processAlive(pid) {
			isAlive = true
		} else {
			status = "stale"
		}
	}
	fields["current_remaining"] = remaining
	fields["is_alive"] = isAlive
	fields["status"] = status
	return fields
}

var errSessionNotFound = errors.New("session not found")

func stopSession(name string) error {
	session, found, err := state.GetSession(name)
	if err != nil {
		return err
	}
	if !found {
		return errSessionNotFound
	}

	tmuxSession := stringField(session.Fields, "tmux_session")
	pid, _ := intField(session.Fields, "pid")

	if tmuxSession != "" {
		_ = exec.Command("tmux", "kill-session", "-t", tmuxSession).Run()
	} else if pid > 0 && processAlive(pid) {
		if proc, err := findProcess(pid); err == nil {
			_ = proc.Signal(syscall.SIGTERM)
		}
	}

	return state.SetSession(name, map[string]interface{}{
		"status":       "stopped",
		"pid":          0,
		"tmux_session": "",
	})
}

func writeJSON(w http.ResponseWriter, status int, payload interface{}) {
	data, err := json.Marshal(payload)
	if err != nil {
		writeJSONError(w, http.StatusInternalServerError, "Failed to encode response")
		return
	}
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	_, _ = w.Write(data)
}

func writeJSONError(w http.ResponseWriter, status int, message string) {
	payload := map[string]string{"error": message}
	writeJSON(w, status, payload)
}

func ensureNameField(name string, fields map[string]interface{}) map[string]interface{} {
	if fields == nil {
		fields = map[string]interface{}{}
	}
	fields["name"] = name
	return fields
}

func copyFields(fields map[string]interface{}) map[string]interface{} {
	if fields == nil {
		return map[string]interface{}{}
	}
	copied := make(map[string]interface{}, len(fields))
	for key, value := range fields {
		copied[key] = value
	}
	return copied
}

func stringField(fields map[string]interface{}, key string) string {
	if fields == nil {
		return ""
	}
	value, ok := fields[key]
	if !ok || value == nil {
		return ""
	}
	switch typed := value.(type) {
	case string:
		return typed
	case fmt.Stringer:
		return typed.String()
	default:
		return fmt.Sprint(value)
	}
}

func intField(fields map[string]interface{}, key string) (int, bool) {
	if fields == nil {
		return 0, false
	}
	value, ok := fields[key]
	if !ok || value == nil {
		return 0, false
	}
	switch typed := value.(type) {
	case int:
		return typed, true
	case int64:
		return int(typed), true
	case float64:
		return int(typed), true
	case json.Number:
		parsed, err := typed.Int64()
		if err != nil {
			return 0, false
		}
		return int(parsed), true
	case string:
		parsed, err := strconv.Atoi(typed)
		if err != nil {
			return 0, false
		}
		return parsed, true
	default:
		return 0, false
	}
}

func processAlive(pid int) bool {
	if pid <= 0 {
		return false
	}
	return syscall.Kill(pid, 0) == nil
}

type osProcess interface {
	Signal(sig os.Signal) error
}

func findProcess(pid int) (osProcess, error) {
	return osFindProcess(pid)
}

var osFindProcess = func(pid int) (osProcess, error) {
	return os.FindProcess(pid)
}
