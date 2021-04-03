program Increment;

var
  ix: integer;
  sum: real;

begin
  sum := 0.0;
  while ix < 10 do begin
    sum := sum + 0.1;
    writeln_real(sum);
    ix := ix + 1
  end;
end.
