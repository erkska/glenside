#include "mlp.c"
#include <assert.h>
#include <math.h>
#include <stdio.h>

// Generated with mlp_c_gen_helper.py
float input[2] = {0.07359921, 0.77422889};
float weights_0[2][4] = {{0.8101289, 0.6391721, 0.38471687, 0.43476797},
                         {0.19664564, 0.67206388, 0.23291401, 0.59172156}};
float weights_1[4][6] = {
    {0.98278998, 0.61484222, 0.67881745, 0.93680619, 0.86837485, 0.42437781},
    {0.45952102, 0.15076425, 0.52289059, 0.45032712, 0.80552555, 0.3883881},
    {0.08942791, 0.76285151, 0.6280041, 0.31396817, 0.80541261, 0.86247733},
    {0.28771746, 0.1725068, 0.18405514, 0.33522804, 0.59388266, 0.53529322}};
float weights_2[6][2] = {{0.5403619, 0.34246081}, {0.96203926, 0.50463116},
                         {0.21667461, 0.7079291}, {0.04013655, 0.3549699},
                         {0.4628509, 0.52579893}, {0.28693871, 0.41747579}};
float out[2] = {0., 0.};
float expected_out[2] = {1.67773846, 2.05100039};

int main() {
  mlp(out, input, weights_0, weights_1, weights_2);

  // Ensure result is what we expect.
  int dim_0;
  for (dim_0 = 0; dim_0 < 2; ++dim_0) {
    fprintf(stderr, "%f ?= %f\\n", out[dim_0],expected_out[dim_0]);
    assert(fabs(out[dim_0] - expected_out[dim_0]) < 0.00001);
  }

  return 0;
}
