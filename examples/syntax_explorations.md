[fn add_one [x] [+ x 1]]

(define (add_one (lambda (x) (+ x 1))))))))))

(fun add_one (x) (+ x 1))

- fn:
    name: add_one
    formals:
      - x
    body:
      - "+":
          - "$": x
          - 1
      - return:


- fn:
    greeting:
      - subject
    in:
      - println: "hello, "
      - println: "$subject"
- greeting: "world"


- import: server
- serve:
  - /banana: "banana"
  - /strawberry: "strawberry"
- banana:
    - respond: "banana"

