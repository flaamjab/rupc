program Fibonacci;

var
  x, y, z: integer;

begin
  x := 0;
  y := 1;

  while x < 255 do begin
    writeln_int(x);
    z := x + y;
    x := y;
    y := z
  end;
end.
