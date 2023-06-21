(defstruct parser
  "The state of a parser"
  'stack   (vec)
  'indices (vec)
  'src     src
  'idx     idx 
  'line    0
  'column  0)

(defun parser/get-pos (state)
  (cons (parser/line state) (parser/column state)))

(defun parser/open (state)
  "Open a new scope for a list inside the parser"
  (vec/push (parser/indices state)
            (cons (len stack)
                  (parser/get-pos state))))

(defun parser/close (state)
  "Closes a scope for a list and pushes it to the stack"  
  (if (vec/is-empty indices)
      (throw :unclosed-parenthesis)
      (block 
        (let info (vec/pop (parser/indices state)))
        (let args (vec/split-off stack (info/index info)))
        (let expr (vec/rfold 'nil (lambda (y x) (list 'cons x y)))
        (vec/push (parser/stack state) expr)))))

(defun parser/inc-line (state)
  "Increments the line number"
  (parser/set-line state (lambda (x) (+ x 1))))

(defun parser/inc-column (state)
  "Increments the column number"
  (parser/set-column state (lambda (x) (+ x 1))))

(defun parser/inc-idx (state char)
  "Increments the index number"
  (parser/set-idx state (lambda (x) + (str/utf8-len char) x)))

(defun parser/next-char (state)
  "Jumps a character and returns it"
  (let chr (str/first-chr state))
  (unless (eq chr nil)
    (parser/inc-idx chr)
    (cond chr
      ("\n" (block
        (parser/inc-line state)
        (parser/set-column state 0)))
      (otherwise 
        (parser/inc-column state))))
  (chr))

(let whitespace (vec "\n" "\r" "\t" " "))

(defun parse (src)
  "Parses a string into a S-Expr"
  (let char (parser/next-char state))
  (while (not (is-nil char))
    (let place (state/get-pos state))
    (case 'true
      ((vec/contains whitespace chr) (nil))
      ((= chr "(") (state/open state))
      ((= chr ")") (state/close state))
      ((not (whitespace) chr) (block
        (let str (clone chr))
        (while (not (whitespace (parser/peek state)))
          (str/push str (parser/next-char state)))
        (vec/push (state/indices state) (map 'type 'identifier 'value str)))))))
